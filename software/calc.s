; VINTAGE-1
; Author: roywalk3r
; Repo: https://github.com/roywalk3r/vintage
; License: MIT
; calc.s - the first VINTAGE-1 application: a 16-bit integer calculator.
; Digits 0-9 enter a number, + - * / chain left-to-right, = or Enter
; computes, C resets, Backspace rubs out a digit. Division by zero raises
; an ERR state cleared by the next digit or C. Entry clamps at 65535
; (an overflowing digit is ignored); add/sub/mul wrap modulo 65536.
; The display field (op glyph + 14 number cells) is mirrored in RAM at
; $2010 as ASCII for tooling and headless tests.

        .org $E000
SCREEN = $4000

; --- zero page ---
; hello.s draw_msg working set re-used verbatim at $E0-$E9
MSGLO = $E0
MSGHI = $E1
DLO   = $E2
DHI   = $E3
FLO   = $E2
FHI   = $E3
GLYPH = $E6
GHI   = $E7
CHIDX = $E8
VAL   = $E9

ENTRYH  = $13          ; hi byte of ENTRY
ENTRY   = $12          ; 16-bit number being typed
ACCH    = $15          ; hi byte of ACC
ACC     = $14          ; 16-bit running accumulator
COMMH   = $17          ; hi byte of COMM
COMM    = $16          ; 16-bit committed operand
DVND    = $18          ; 16-bit dividend fed to div16
DVNDH   = $19
M2H     = $1B
M2      = $1A          ; 16-bit divisor / multiplier bit source
M1H     = $1D
M1      = $1C          ; 16-bit shifting addend for mul16
RESH    = $1F
RES     = $1E          ; 16-bit mul result
REMH    = $21
REM     = $20          ; div16 remainder
QUOH    = $23
QUO     = $22          ; div16 quotient
T0H     = $25
T0      = $24
T1H     = $27
T1      = $26
DLEN    = $28          ; digits collected, LSB-first in DBUF
DESTR   = $29          ; field offset of the first digit
ENTRYF  = $2A          ; entry mode flag
OP      = $2B          ; pending operator: 0 none 1 add 2 sub 3 mul 4 div
ERRF    = $2C
TK      = $2F          ; last key posted from $5800
BOP     = $30          ; operator handed to apply()
DISPL   = $32
DISPH   = $33

FIELD   = $2010        ; 15-byte ASCII display field, $2010=$20 15 chars + 0
DBUF    = $2050        ; decimal-digit scratch, LSB-first

; --- boot: lay out the static screen, then poll the keyboard ---
start:  jsr clear_scr
        lda #<titlemsg
        sta MSGLO
        lda #titlemsg/$100
        sta MSGHI
        lda #<TADDR
        sta DLO
        lda #TADDR/$100
        sta DHI
        jsr draw_msg
        lda #<rulemsg
        sta MSGLO
        lda #rulemsg/$100
        sta MSGHI
        lda #<RADDR
        sta DLO
        lda #RADDR/$100
        sta DHI
        jsr draw_msg
        lda #<legend
        sta MSGLO
        lda #legend/$100
        sta MSGHI
        lda #<HADDR
        sta DLO
        lda #HADDR/$100
        sta DHI
        jsr draw_msg
        lda #<legend2
        sta MSGLO
        lda #legend2/$100
        sta MSGHI
        lda #<L2ADDR
        sta DLO
        lda #L2ADDR/$100
        sta DHI
        jsr draw_msg
        lda #0
        sta ENTRY
        sta ENTRYH
        lda #0
        sta ENTRYF
        sta OP
        sta ERRF
        jsr render
poll:   lda $5800
        beq poll
        jsr handle
        jsr render
        jmp poll

; --- key dispatch: A holds the raw $5800 byte -------------------------
handle: sta TK
        lda TK
        cmp #$08
        beq hbks
        cmp #$43
        beq hclr_j
        cmp #$63
        beq hclr_j
        cmp #$30
        bcc hops
        cmp #$3A
        bcc hdigit
hops:
        lda TK
        cmp #$2A
        beq hmul_j
        cmp #$2B
        beq hadd_j
        cmp #$15
        beq hadd_j
        cmp #$2D
        beq hsub_j
        cmp #$16
        beq hsub_j
        cmp #$2F
        beq hdiv_j
        cmp #$3D
        beq heq_j
        cmp #$0D
        beq heq_j
        rts

; branch-range trampolines
hclr_j: jmp hclr
hadd_j: jmp hadd
hsub_j: jmp hsub
hmul_j: jmp hmul
hdiv_j: jmp hdiv
heq_j:  jmp heq
hbks:
        lda ENTRYF
        beq hdone
        lda ENTRY
        sta DVND
        lda ENTRYH
        sta DVNDH
        lda #10
        sta M2
        lda #0
        sta M2H
        jsr div16
        lda QUO
        sta ENTRY
        lda QUOH
        sta ENTRYH
hdone:  rts

hdigit:
        lda #0
        sta ERRF
        lda ENTRYF
        bne hd1
        lda #0
        sta ENTRY
        sta ENTRYH
        lda #1
        sta ENTRYF
hd1:
; ENTRY = ENTRY*10 + d  (2E+8E): ignore the key on 16-bit overflow
        lda ENTRY
        sta T1
        lda ENTRYH
        sta T1H
        asl T1          ; T1 = 2E
        rol T1H
        bcs hdign
        lda T1
        sta T0          ; T0 = 2E (the +2 part)
        lda T1H
        sta T0H
        asl T1          ; T1 = 4E
        rol T1H
        bcs hdign
        asl T1          ; T1 = 8E
        rol T1H
        bcs hdign
        lda T1
        clc
        adc T0
        sta T0          ; T0 = 2E+8E = 10E
        lda T0H
        adc T1H
        sta T0H
        bcs hdign
        lda TK
        sec
        sbc #$30
        clc
        adc T0
        sta T0
        lda #0
        adc T0H
        sta T0H
        bcs hdign
        lda T0
        sta ENTRY
        lda T0H
        sta ENTRYH
hdign:  rts

hclr:   lda #0
        sta ENTRY
        sta ENTRYH
        sta ACC
        sta ACCH
        sta ENTRYF
        sta OP
        sta ERRF
        rts

 ; op keys: load BOP with the op number, then apply the pending operator
hadd:   lda #1
        sta BOP
        jsr apply
        rts
hsub:   lda #2
        sta BOP
        jsr apply
        rts
hmul:   lda #3
        sta BOP
        jsr apply
        rts
hdiv:   lda #4
        sta BOP
        jsr apply
        rts
heq:    lda #0         ; '=' clears the pending op: the display blanks the
        sta BOP        ; op cell and the next operand starts fresh
        jsr apply
        rts

; --- apply: fold the committed operand into ACC via the pending op ------
apply:  lda ENTRYF
        beq apacc
        lda ENTRY
        sta COMM
        lda ENTRYH
        sta COMMH
        lda #0
        sta ENTRYF
        jmp apchk
apacc:  lda ACC
        sta COMM
        lda ACCH
        sta COMMH
apchk:  lda OP
        beq apcopy_j
        lda ACC
        sta M1
        lda ACCH
        sta M1H
        lda COMM
        sta M2
        lda COMMH
        sta M2H
        lda OP
        cmp #1
        beq apadd
        cmp #2
        beq apsub
        cmp #3
        beq apmul
        cmp #4
        bne apcopy_j
        jmp apdiv
apcopy_j: jmp apcopy
apadd:  lda COMM
        clc
        adc ACC
        sta ACC
        lda COMMH
        adc ACCH
        sta ACCH
        jmp apset
apsub:  lda ACC
        sec
        sbc COMM
        sta ACC
        lda ACCH
        sbc COMMH
        sta ACCH
        jmp apset
apmul:  jsr mul16
        lda RES
        sta ACC
        lda RESH
        sta ACCH
        jmp apset
apdiv:  lda COMM
        bne apdivx
        lda COMMH
        bne apdivx
        lda #1
        sta ERRF
        jmp apset
apdivx: lda ACC
        sta DVND
        lda ACCH
        sta DVNDH
        jsr div16
        lda QUO
        sta ACC
        lda QUOH
        sta ACCH
apset:  lda BOP
        sta OP
        rts
apcopy: lda COMM
        sta ACC
        lda COMMH
        sta ACCH
        jmp apset

; --- mul16: RES = M1 * M2 (low 16 bits), clobbers M1, M2 ----------------
mul16:  lda #0
        sta RES
        sta RESH
        ldx #16
mloop:  lsr M2H
        ror M2
        bcc mshift
        lda M1
        clc
        adc RES
        sta RES
        lda M1H
        adc RESH
        sta RESH
mshift: asl M1
        rol M1H
        dex
        bne mloop
        rts

; --- div16: QUO = DVND / M2, REM = remainder; M2 preserved --------------
div16:  lda #0
        sta QUO
        sta QUOH
        sta REM
        sta REMH
        ldx #16
dloop:  asl DVND
        rol DVNDH
        rol REM
        rol REMH
        lda REM
        cmp M2
        lda REMH
        sbc M2H
        bcc dshift0
        lda REM
        sbc M2
        sta REM
        lda REMH
        sbc M2H
        sta REMH
        sec
        jmp dshift
dshift0:
        clc
dshift: rol QUO
        rol QUOH
        dex
        bne dloop
        rts

; --- render: rebuild the 15-byte display field and blit it -------------
render: lda #$20
        ldx #14
rbl:    sta FIELD,x
        dex
        bpl rbl
        lda ERRF
        beq rop
        lda #$45
        sta FIELD+12
        lda #$52
        sta FIELD+13
        sta FIELD+14
        jmp rdraw
rop:    ldx OP
        beq rnum
        lda OPTAB,x
        sta FIELD
rnum:   lda ENTRYF
        beq ruseacc
        lda ENTRY
        sta DISPL
        lda ENTRYH
        sta DISPH
        jmp rdec
ruseacc:
        lda ACC
        sta DISPL
        lda ACCH
        sta DISPH
rdec:   jsr to_dec
        lda #15
        sec
        sbc DLEN
        tax
        ldy DLEN
rdl:    dey
        lda DBUF,y      ; DBUF is LSB-first; walk it backwards so the
        sta FIELD,x     ; most significant digit lands leftmost
        inx
        cpy #0
        bne rdl
        jmp rdraw
rdraw:  lda #<FIELD
        sta MSGLO
        lda #FIELD/$100
        sta MSGHI
        lda #<FADDR
        sta DLO
        lda #FADDR/$100
        sta DHI
        jsr draw_msg
        rts

; --- to_dec: DISPL/H -> decimal digits LSB-first in DBUF, count in DLEN -
to_dec: lda #0
        sta DLEN
tdl:    lda DISPL
        sta DVND
        lda DISPH
        sta DVNDH
        lda #10
        sta M2
        lda #0
        sta M2H
        jsr div16
        lda REM
        ora #$30       ; remainder is 0-9, make it ASCII
        ldy DLEN
        sta DBUF,y
        iny
        sty DLEN
        lda QUO
        sta DISPL
        lda QUOH
        sta DISPH
        lda QUO
        ora QUOH
        bne tdl
        rts

; screen addresses
TADDR = SCREEN + 2*256 + 9     ; title, row 2, col 9
FADDR = SCREEN + 4*256 + 7     ; display field, row 4, cols 7-21
RADDR = SCREEN + 6*256 + 7     ; rule, row 6
HADDR = SCREEN + 9*256 + 7     ; key legend, row 9
L2ADDR = SCREEN + 10*256 + 7   ; legend line 2, row 10

; --- draw_msg: null-terminated text at DLO/DHI, glyphs from FONT --------
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
        sta GHI
        lda #<FONT
        clc
        adc GLYPH
        sta FLO
        lda #FONT/$100
        adc GHI
        sta FHI
        ldx #0
rloop:  txa
        tay
        lda (FLO),y
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
        bcs nc
        dec DHI
nc:     inc CHIDX
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

; --- static strings ----------------------------------------------------
titlemsg:
        .text "VINTAGE-1 CALC"
        .byte 0
rulemsg:
        .text "______________"
        .byte 0
legend:
        .text "0-9 + - * / ="
        .byte 0
legend2:
        .text "C CLEARS BKSP"
        .byte 0

OPTAB:  .byte $20,$2B,$2D,$2A,$2F ; op index -> glyph (0=blank)

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
