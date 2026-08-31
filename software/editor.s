; VINTAGE-1
; Author: roywalk3r
; Repo: https://github.com/roywalk3r/vintage
; License: MIT
; editor.s - the scratchpad: a fixed 8-line x 28-column text editor.
; Arrows (or W A S D) move the cursor, printable keys insert at the
; cursor, Backspace rubs out to the left, Enter drops to the next line
; at column 0. The cursor blinks by redrawing its cell every frame with
; the glyph inverted, gated on the $5802 frame counter. Text lives in
; RAM at $1000, 8 lines of 28 bytes plus a NUL — the contract headless
; tests assert on.

        .org $E000
SCREEN = $4000
TXT    = $1000         ; 8 lines x (28 chars + NUL)

; --- zero page ($12-$24) ---
CX    = $12            ; cursor column 0..27
CY    = $13            ; cursor line 0..7
CURV  = $14            ; cursor cell currently inverted
BLK   = $15            ; last frame-counter low harmless: never read
TK    = $16            ; last raw key
IDX   = $17            ; scratch index
LIDX  = $18            ; line index for render
LP    = $20            ; pointer to the current line
LPH   = $21
TP    = $22            ; temp pointer (LP-1 during delete)
TPH   = $23
CHINV = $24            ; $FF inverts glyphs (cursor cell)

; draw_msg working set, $E0-$E9
MSGLO = $E0
MSGHI = $E1
DLO   = $E2
DHI   = $E3
FLO   = $E4            ; font pointer: separate from DLO, whose cell the
FHI   = $E5            ; glyph loop must not clobber while it is writing
GLYPH = $E6
GH    = $E7
CHIDX = $E8
VAL   = $E9

CHBUF = $1200          ; 1-char draw buffer, second byte stays NUL

; --- boot: static chrome, empty buffer, then the poll/blink loop -------
start:  jsr clear_scr
        lda #<title
        sta MSGLO
        lda #title/$100
        sta MSGHI
        lda #<TADDR
        sta DLO
        lda #TADDR/$100
        sta DHI
        jsr draw_msg
        lda #0
        ldx #0
wzero:  sta TXT,x       ; 8 x 29 = 232 bytes span $1000-$10E7
        inx
        bne wzero
        lda #0
        sta CX
        sta CY
        sta CURV
        sta BLK
        jsr render
poll:   lda $5802
        cmp BLK
        beq nokf
        sta BLK
        jsr blink       ; once per frame: flip the cursor cell
nokf:   lda $5800
        beq poll
        jsr handle
        jsr render
        jmp poll
; --- key dispatch: A holds the raw $5800 byte -------------------------
 ; branch trampolines (up/down/insert sit beyond +-127 branches)
hup_j:  jmp hup
hdn_j:  jmp hdn
handle: sta TK
        cmp #$11
        beq hup_j
        cmp #$12
        beq hdn_j
        cmp #$13
        beq hlt
        cmp #$14
        beq hrt
        cmp #$08
        beq hdel_j
        cmp #$0D
        beq hent_j
        cmp #$15
        beq hplus_j
        cmp #$16
        beq hminus_j
        cmp #$20
        bcc hign
        cmp #$7F
        bcc hins_j2
hign:   rts
hdel_j: jmp hdel
hent_j: jmp hent
hplus_j: jmp hplus
hminus_j: jmp hminus
hins_j2: jmp hins
hup:    lda CY
        beq hdone
        dec CY
        rts
hdn:    lda CY
        cmp #7
        bcc hdn2
        rts
hdn2:   inc CY
        rts
hlt:    lda CX
        beq hdone
        dec CX
        rts
hrt:    lda CX
        cmp #27
        bcs hdone
        inc CX
        rts
hdone:  rts
hent:   lda CY
        cmp #7
        bcs hdone
        inc CY
        lda #0
        sta CX
        rts
 ; $15/$16 arrive when the typist presses +/- (calc's contract); the
 ; editor inserts the same glyphs
hplus:  lda #$2B
        sta TK
        jmp hins
hminus: lda #$2D
        sta TK
 ; fall through into hins
hins:   jsr linelp
        ldy #0
hp0:    cpy CX
        bcs hins1        ; padded up to the cursor
        lda (LP),y
        bne hp1
        lda #$20        ; a NUL before the cursor: pad the gap with spaces
hp1:    sta (LP),y
        iny
        jmp hp0
hins1:  lda (LP),y
        beq hins2
        iny
        cpy #29
        bcc hins1
        rts
hins2:  cpy #28
        bcc hins3
        rts
hins3:  sta IDX         ; index of the NUL
        lda LP
        clc
        adc #1
        sta TP
        lda #0
        adc LPH
        sta TPH         ; TP = LP+1: the shift lands one cell up
        ldy CX
hins4:  lda (LP),y
        sta (TP),y      ; b[y+1] = b[y]
        iny
        cpy IDX
        bcc hins4
        ldy CX
        lda TK
        sta (LP),y
        inc CX
        lda CX
        cmp #28
        bcc hdone
        dec CX
        rts

 ; backspace: close up the cell left of the cursor; at column 0 it is
 ; a no-op (no line joining)
hdel:   lda CX
        beq hdone
        jsr linelp
        lda LP
        sta TP
        lda LPH
        sta TPH
        lda TP
        bne hdel2
        dec TPH
hdel2:  dec TP          ; TP = LP - 1: the delete lands one cell left
        ldy CX
hdel3:  lda (LP),y
        sta (TP),y
        beq hdel9      ; copied the NUL: tail reattached
        iny
        jmp hdel3
hdel9:  dec CX          ; the cursor follows the deleted cell
        rts

 ; LP/LPH -> start of line CY
linelp: ldy CY
        lda LPTLO,y
        sta LP
        lda LPTHI,y
        sta LPH
        rts

 ; insert pads any NUL gap before the cursor with spaces, so a cursor
 ; parked past the line's text (down-arrow with a kept column) still
 ; lands on a real cell and every line stays one NUL-terminated string

; --- render: rebuild every text row, then the cursor cell --------------
render: lda #0
        sta CHINV
        lda #0
        sta LIDX
rlp:    ldy LIDX
        lda LPTLO,y
        sta MSGLO
        lda LPTHI,y
        sta MSGHI
        lda #0
        sta DLO
        tya
        clc
        adc #$42        ; text rows are screen rows 2-9
        sta DHI
        jsr draw_msg
        inc LIDX
        lda LIDX
        cmp #8
        bne rlp
        jsr curcell     ; cursor cell drawn last
        rts

 ; draw the cursor cell: the glyph under (CX, CY), inverted iff
 ; CHINV = $FF; the scan never reads past the line's NUL
curcell:
        jsr linelp
        ldy #0
cc0:    cpy CX
        beq cc2
        lda (LP),y
        beq ccS
        iny
        bne cc0
ccS:    lda #$20
        bne cc1
cc2:    ldy CX
        lda (LP),y
        bne cc1
        lda #$20
cc1:    sta CHBUF
        lda CX
        sta DLO
        lda CY
        clc
        adc #$42
        sta DHI
        lda #<CHBUF
        sta MSGLO
        lda #CHBUF/$100
        sta MSGHI
        jmp draw_msg

 ; blink: flip CURV, then redraw the cursor cell in the new polarity
blink:  lda CURV
        eor #1
        sta CURV
        ldx CURV
        beq bl1
        lda #$FF
        sta CHINV
        jmp bl2
bl1:    lda #0
        sta CHINV
bl2:    jmp curcell

 ; --- draw_msg: null-terminated text at DLO/DHI, glyphs from FONT ------
draw_msg:
        lda #0
        sta CHIDX
chloop:
        ldy CHIDX
        lda (MSGLO),y
        beq dmdone
        sec
        sbc #$20
        sta GLYPH
        lda #0
        ldx #3
shl:    asl GLYPH
        rol a
        dex
        bne shl
        sta GH
        lda #<FONT
        clc
        adc GLYPH
        sta FLO
        lda #FONT/$100
        adc GH
        sta FHI
        ldx #0
rloop:  txa
        tay
        lda (FLO),y
        eor CHINV
        ldy #0
        sta (DLO),y
        lda DLO
        clc
        adc #32
        sta DLO
        bcc nr
        inc DHI
nr:     inx
        cpx #8
        bne rloop
        lda DLO
        sec
        sbc #$FF
        sta DLO
        bcs nc1
        dec DHI
nc1:    inc CHIDX
        jmp chloop
dmdone:
        rts

 ; --- clear the framebuffer --------------------------------------------
clear_scr:
        lda #0
        ldx #0
cloop:  sta $4000,x
        sta $4100,x
        sta $4200,x
        sta $4300,x
        sta $4400,x
        sta $4500,x
        sta $4600,x
        sta $4700,x
        sta $4800,x
        sta $4900,x
        sta $4A00,x
        sta $4B00,x
        sta $4C00,x
        sta $4D00,x
        sta $4E00,x
        sta $4F00,x
        sta $5000,x
        sta $5100,x
        sta $5200,x
        sta $5300,x
        sta $5400,x
        sta $5500,x
        sta $5600,x
        sta $5700,x
        inx
        bne cloop
        rts

; --- static data -------------------------------------------------------
title:
        .text "VINTAGE-1 EDIT"
        .byte 0

TADDR = SCREEN         ; title at row 0, col 0
; 8 lines x 29 bytes at $1000: line L starts at $1000 + L*29
LPTLO:  .byte $00,$1D,$3A,$57,$74,$91,$AE,$CB
LPTHI:  .byte $10,$10,$10,$10,$10,$10,$10,$10

stub:   rti

        .org $FFFA
        .word stub, start, stub
.org $F000
FONT:
 .byte $00,$00,$00,$00,$00,$00,$00,$00 ; `$20`
 .byte $18,$3C,$3C,$18,$18,$00,$18,$00 ; `$21`
 .byte $6C,$6C,$00,$00,$00,$00,$00,$00 ; `$22`
 .byte $6C,$6C,$FE,$6C,$FE,$6C,$6C,$00 ; `$23`
 .byte $30,$7C,$C0,$78,$0C,$F8,$30,$00 ; `$24`
 .byte $00,$C6,$CC,$18,$30,$66,$C6,$00 ; `$25`
 .byte $38,$6C,$38,$76,$DC,$CC,$76,$00 ; `$26`
 .byte $60,$60,$C0,$00,$00,$00,$00,$00 ; `$27`
 .byte $18,$30,$60,$60,$60,$30,$18,$00 ; `$28`
 .byte $60,$30,$18,$18,$18,$30,$60,$00 ; `$29`
 .byte $00,$66,$3C,$FF,$3C,$66,$00,$00 ; `$2A`
 .byte $00,$30,$30,$FC,$30,$30,$00,$00 ; `$2B`
 .byte $00,$00,$00,$00,$00,$30,$30,$60 ; `$2C`
 .byte $00,$00,$00,$FC,$00,$00,$00,$00 ; `$2D`
 .byte $00,$00,$00,$00,$00,$30,$30,$00 ; `$2E`
 .byte $06,$0C,$18,$30,$60,$C0,$80,$00 ; `$2F`
 .byte $7C,$C6,$CE,$DE,$F6,$E6,$7C,$00 ; '0'
 .byte $30,$70,$30,$30,$30,$30,$FC,$00 ; '1'
 .byte $78,$CC,$0C,$38,$60,$CC,$FC,$00 ; '2'
 .byte $78,$CC,$0C,$38,$0C,$CC,$78,$00 ; '3'
 .byte $1C,$3C,$6C,$CC,$FE,$0C,$1E,$00 ; '4'
 .byte $FC,$C0,$F8,$0C,$0C,$CC,$78,$00 ; '5'
 .byte $38,$60,$C0,$F8,$CC,$CC,$78,$00 ; '6'
 .byte $FC,$CC,$0C,$18,$30,$30,$30,$00 ; '7'
 .byte $78,$CC,$CC,$78,$CC,$CC,$78,$00 ; '8'
 .byte $78,$CC,$CC,$7C,$0C,$18,$70,$00 ; '9'
 .byte $00,$30,$30,$00,$00,$30,$30,$00 ; `$3A`
 .byte $00,$30,$30,$00,$00,$30,$30,$60 ; `$3B`
 .byte $18,$30,$60,$C0,$60,$30,$18,$00 ; `$3C`
 .byte $00,$00,$FC,$00,$00,$FC,$00,$00 ; `$3D`
 .byte $60,$30,$18,$0C,$18,$30,$60,$00 ; `$3E`
 .byte $78,$CC,$0C,$18,$30,$00,$30,$00 ; `$3F`
 .byte $7C,$C6,$DE,$DE,$DE,$C0,$78,$00 ; `$40`
 .byte $30,$78,$CC,$CC,$FC,$CC,$CC,$00 ; 'A'
 .byte $FC,$66,$66,$7C,$66,$66,$FC,$00 ; 'B'
 .byte $3C,$66,$C0,$C0,$C0,$66,$3C,$00 ; 'C'
 .byte $F8,$6C,$66,$66,$66,$6C,$F8,$00 ; 'D'
 .byte $FE,$62,$68,$78,$68,$62,$FE,$00 ; 'E'
 .byte $FE,$62,$68,$78,$68,$60,$F0,$00 ; 'F'
 .byte $3C,$66,$C0,$C0,$CE,$66,$3E,$00 ; 'G'
 .byte $CC,$CC,$CC,$FC,$CC,$CC,$CC,$00 ; 'H'
 .byte $78,$30,$30,$30,$30,$30,$78,$00 ; 'I'
 .byte $1E,$0C,$0C,$0C,$CC,$CC,$78,$00 ; 'J'
 .byte $E6,$66,$6C,$78,$6C,$66,$E6,$00 ; 'K'
 .byte $F0,$60,$60,$60,$62,$66,$FE,$00 ; 'L'
 .byte $C6,$EE,$FE,$FE,$D6,$C6,$C6,$00 ; 'M'
 .byte $C6,$E6,$F6,$DE,$CE,$C6,$C6,$00 ; 'N'
 .byte $38,$6C,$C6,$C6,$C6,$6C,$38,$00 ; 'O'
 .byte $FC,$66,$66,$7C,$60,$60,$F0,$00 ; 'P'
 .byte $78,$CC,$CC,$CC,$DC,$78,$1C,$00 ; 'Q'
 .byte $FC,$66,$66,$7C,$6C,$66,$E6,$00 ; 'R'
 .byte $78,$CC,$E0,$70,$1C,$CC,$78,$00 ; 'S'
 .byte $FC,$B4,$30,$30,$30,$30,$78,$00 ; 'T'
 .byte $CC,$CC,$CC,$CC,$CC,$CC,$FC,$00 ; 'U'
 .byte $CC,$CC,$CC,$CC,$CC,$78,$30,$00 ; 'V'
 .byte $C6,$C6,$C6,$D6,$FE,$EE,$C6,$00 ; 'W'
 .byte $C6,$C6,$6C,$38,$38,$6C,$C6,$00 ; 'X'
 .byte $CC,$CC,$CC,$78,$30,$30,$78,$00 ; 'Y'
 .byte $FE,$C6,$8C,$18,$32,$66,$FE,$00 ; 'Z'
 .byte $78,$60,$60,$60,$60,$60,$78,$00 ; `$5B`
 .byte $C0,$60,$30,$18,$0C,$06,$02,$00 ; `$5C`
 .byte $78,$18,$18,$18,$18,$18,$78,$00 ; `$5D`
 .byte $10,$38,$6C,$C6,$00,$00,$00,$00 ; `$5E`
 .byte $00,$00,$00,$00,$00,$00,$00,$FF ; `$5F`
 .byte $30,$30,$18,$00,$00,$00,$00,$00 ; `$60`
 .byte $00,$00,$78,$0C,$7C,$CC,$76,$00 ; 'a'
 .byte $E0,$60,$60,$7C,$66,$66,$DC,$00 ; 'b'
 .byte $00,$00,$78,$CC,$C0,$CC,$78,$00 ; 'c'
 .byte $1C,$0C,$0C,$7C,$CC,$CC,$76,$00 ; 'd'
 .byte $00,$00,$78,$CC,$FC,$C0,$78,$00 ; 'e'
 .byte $38,$6C,$60,$F0,$60,$60,$F0,$00 ; 'f'
 .byte $00,$00,$76,$CC,$CC,$7C,$0C,$F8 ; 'g'
 .byte $E0,$60,$6C,$76,$66,$66,$E6,$00 ; 'h'
 .byte $30,$00,$70,$30,$30,$30,$78,$00 ; 'i'
 .byte $0C,$00,$0C,$0C,$0C,$CC,$CC,$78 ; 'j'
 .byte $E0,$60,$66,$6C,$78,$6C,$E6,$00 ; 'k'
 .byte $70,$30,$30,$30,$30,$30,$78,$00 ; 'l'
 .byte $00,$00,$CC,$FE,$FE,$D6,$C6,$00 ; 'm'
 .byte $00,$00,$F8,$CC,$CC,$CC,$CC,$00 ; 'n'
 .byte $00,$00,$78,$CC,$CC,$CC,$78,$00 ; 'o'
 .byte $00,$00,$DC,$66,$66,$7C,$60,$F0 ; 'p'
 .byte $00,$00,$76,$CC,$CC,$7C,$0C,$1E ; 'q'
 .byte $00,$00,$DC,$76,$66,$60,$F0,$00 ; 'r'
 .byte $00,$00,$7C,$C0,$78,$0C,$F8,$00 ; 's'
 .byte $10,$30,$7C,$30,$30,$34,$18,$00 ; 't'
 .byte $00,$00,$CC,$CC,$CC,$CC,$76,$00 ; 'u'
 .byte $00,$00,$CC,$CC,$CC,$78,$30,$00 ; 'v'
 .byte $00,$00,$C6,$D6,$FE,$FE,$6C,$00 ; 'w'
 .byte $00,$00,$C6,$6C,$38,$6C,$C6,$00 ; 'x'
 .byte $00,$00,$CC,$CC,$CC,$7C,$0C,$F8 ; 'y'
 .byte $00,$00,$FC,$98,$30,$64,$FC,$00 ; 'z'
 .byte $1C,$30,$30,$E0,$30,$30,$1C,$00 ; `$7B`
 .byte $18,$18,$18,$00,$18,$18,$18,$00 ; `$7C`
 .byte $E0,$30,$30,$1C,$30,$30,$E0,$00 ; `$7D`
 .byte $76,$DC,$00,$00,$00,$00,$00,$00 ; `$7E`

