; VINTAGE-1 hello ROM — banner text via font8x8 (dhepper/font8x8, public
; domain), rows bit-reversed for the MSB-left framebuffer so each glyph row
; lands in one framebuffer byte.

 .org $E000

SCREEN = $4000
DST1 = SCREEN + 4*32 + 11
DST2 = SCREEN + 12*32 + 6
DST3 = SCREEN + 20*32 + 13

; zero-page pointers
MSGLO = $E0
MSGHI = $E1
FLO   = $E2
FHI   = $E3
DLO   = $E4
DHI   = $E5
GLYPH = $E6
GHI   = $E7
CHIDX = $E8

start:
 jsr clear
 lda #<DST1                  ; "VINTAGE-1", row 8, centred
 sta DLO
 lda #DST1/$100
 sta DHI
 lda #<msg1
 sta MSGLO
 lda #msg1/$100
 sta MSGHI
 jsr draw_msg
 lda #<DST2                 ; "8-BIT DREAM MACHINE", row 10
 sta DLO
 lda #DST2/$100
 sta DHI
 lda #<msg2
 sta MSGLO
 lda #msg2/$100
 sta MSGHI
 jsr draw_msg
 lda #<DST3                 ; "READY.", row 20
 sta DLO
 lda #DST3/$100
 sta DHI
 lda #<msg3
 sta MSGLO
 lda #msg3/$100
 sta MSGHI
 jsr draw_msg
hold: jmp hold

; --- clear the framebuffer ---------------------------------------------
clear:
 lda #0
 ldx #0
cloop:
 sta SCREEN,x
 sta SCREEN+$100,x
 sta SCREEN+$200,x
 sta SCREEN+$300,x
 sta SCREEN+$400,x
 sta SCREEN+$500,x
 sta SCREEN+$600,x
 sta SCREEN+$700,x
 sta SCREEN+$800,x
 sta SCREEN+$900,x
 sta SCREEN+$A00,x
 sta SCREEN+$B00,x
 sta SCREEN+$C00,x
 sta SCREEN+$D00,x
 sta SCREEN+$E00,x
 sta SCREEN+$F00,x
 sta SCREEN+$1000,x
 sta SCREEN+$1100,x
 sta SCREEN+$1200,x
 sta SCREEN+$1300,x
 sta SCREEN+$1400,x
 sta SCREEN+$1500,x
 sta SCREEN+$1600,x
 sta SCREEN+$1700,x
 inx
 bne cloop
 rts

; --- draw a null-terminated message ------------------------------------
; MSGLO/MSGHI = string, DLO/DHI = screen address of first char cell.
; 32x24 grid, 8x8 cells, glyph rows land directly in fb bytes.
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
 lda #0              ; GLYPH*8 as 16-bit: lo in GLYPH, hi in A
 ldx #3
shl: asl GLYPH
 rol a
 dex
 bne shl
 sta GHI
 lda #<FONT
 clc
 adc GLYPH
 sta FLO
 lda #FONT/$100
 adc GHI
 sta FHI
 ldx #0              ; 8 glyph rows
rloop:
 txa
 tay
 lda (FLO),y         ; font row byte
 ldy #0
 sta (DLO),y         ; whole row = one fb byte
 lda DLO             ; advance one scanline (32 bytes)
 clc
 adc #32
 sta DLO
 bcc nr
 inc DHI
nr: inx
 cpx #8
 bne rloop
 lda DLO             ; step back 255: base+256 -> base+1 (next cell)
 sec
 sbc #$FF
 sta DLO
 bcs nc
 dec DHI
nc: inc CHIDX
 jmp chloop
dmdone:
 rts

msg1: .text "VINTAGE-1"
 .byte 0
msg2: .text "8-BIT DREAM MACHINE"
 .byte 0
msg3: .text "READY."
 .byte 0

; --- font8x8 basic latin $20-$7E, rows bit-reversed (MSB-left) ----------
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

 .org $FFFC
 .word start
