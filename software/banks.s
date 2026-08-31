; VINTAGE-1
; Author: roywalk3r
; Repo: https://github.com/roywalk3r/vintage
; License: MIT
; banks.s - a two-bank cartridge showcase. Bank 0 draws vertical bars,
; bank 1 draws horizontal bars, and a dispatcher copied into RAM ($6000)
; flips the cartridge through $5806 every 128 frames. The dispatcher must
; live in RAM: when $5806 changes, the whole $E000-$FFFF window belongs to
; the new bank at once, so a ROM-resident loop cannot survive its switch.

        .org $E000
; Bank 0 screen: vertical bars - $FF on even columns, 0 on odd.
drawa:  lda #$40
        sta $12        ; framebuffer pointer hi
        lda #0
        sta $11        ; and lo
        ldx #192       ; rows
prow:   ldy #0         ; byte column 0..31
pcol:   tya
        and #1
        bne podd
        lda #$FF
        jmp pput
podd:   lda #0
pput:   sta ($11),y
        iny
        cpy #32
        bne pcol
        lda $11        ; advance 32 bytes per scanline
        clc
        adc #32
        sta $11
        bcc pnext
        inc $12
pnext:  dex
        bne prow
        jmp $6000      ; hand back to the RAM dispatcher

 .bank 1
        .org $E000
; Bank 1 screen: horizontal bars - $FF where (y & 7) < 4.
drawb:  lda #$40
        sta $12
        lda #0
        sta $11
        sta $14        ; scanline counter
        lda #192
        sta $15        ; rows remaining
        ldy #0         ; ($11),y index held at 0
hrow:   lda $14
        and #7
        cmp #4
        bcc lit
        lda #0
        beq hfill
lit:    lda #$FF
hfill:  ldx #32
hloop:  sta ($11),y
        inc $11
        bne hstep
        inc $12        ; fb page boundary every 8 lines
hstep:  dex
        bne hloop
        inc $14
        dec $15
        bne hrow
        jmp $6000

 .bank 0
        .org $E070
stub:   rti

 .bank 0
        .org $E080

boot:   ldy #0
copy:   lda disp,y
        sta $6000,y
        iny
        cpy #26
        bne copy
        jmp $6000      ; hand to the RAM copy BEFORE the first bank flip

disp:   ldx #128
dw:     lda $5802
        cmp $10
        beq dw
        sta $10
        dex
        bne dw
        lda $5806
        eor #$01
        sta $5806
        jmp $E000

 .bank 0
        .org $FFFA
        .word stub, boot, stub

 .bank 1
        .org $FFFA
        .word hstub, drawb, hstub
 .bank 1
        .org $E060
hstub:  rti
