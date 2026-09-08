; VINTAGE-1
; Author: roywalk3r
; Repo: https://github.com/roywalk3r/vintage
; License: MIT
; paint.s - keyboard drawing on the 2bpp color framebuffer. Arrows move a
; fat-pixel cursor, z stamps the current color, x erases, c cycles the
; color 1-3, n clears the canvas, s saves the image to a $6000 shadow,
; l restores it. The cursor is the fat pixel under (CURX,CURY) forced to
; palette register value 3: each frame the saved byte is written back and
; the cursor re-stamped, so the image only changes where you draw. The
; contract headless tests assert on is the framebuffer itself.

        .org $E000
SCREEN = $4000          ; framebuffer, 6K
SHADOW = $6000          ; saved-image shadow, 6K of the $6000 RAM bank

; --- zero page ---
CURX   = $12            ; fat-pixel x, 0..127
CURY   = $13            ; fat-pixel y, 0..191
COLOR  = $14            ; press c to cycle 1..3
CURSAV = $15            ; fb byte at the cursor, saved before stamping
BLK    = $16            ; last seen $5802 frame-counter low, gates the loop
FP     = $20            ; fb byte pointer
FPH    = $21
T0     = $24
T1     = $25
T2     = $26

; --- boot: 2bpp mixed scheme, blank canvas, cursor at origin ----------
start:  lda #$83        ; 2bpp color mode, scheme 3
        sta $5804
        jsr fclear
        lda #0
        sta CURX
        sta CURY
        lda #1
        sta COLOR
        jsr stamp       ; save + draw before the first restore
poll:   jsr unstamp
        lda $5800
        beq nokey
        jsr handle
nokey:  jsr stamp
        lda $5802        ; one body per frame: spin until the counter moves
        sta BLK
wait:   lda $5802
        cmp BLK
        beq wait
        jmp poll

; --- key dispatch (A = raw $5800 code) --------------------------------
hup_j:  jmp hup
hdn_j:  jmp hdn
handle: cmp #$11
        beq hup_j
        cmp #$12
        beq hdn_j
        cmp #$13
        beq hlt
        cmp #$14
        beq hrt
        cmp #'z'
        beq hplot
        cmp #'x'
        beq herase
        cmp #'c'
        beq hcol
        cmp #'n'
        beq hclr
        cmp #'s'
        beq hsave
        cmp #'l'
        beq hload
        rts
hup:    lda CURY
        beq hdone
        dec CURY
        rts
hdn:    lda CURY
        cmp #191
        bcc hdn2
        rts
hdn2:   inc CURY
        rts
hlt:    lda CURX
        beq hdone
        dec CURX
        rts
hrt:    lda CURX
        cmp #127
        bcc hrt2
        rts
hrt2:   inc CURX
        rts
hdone:  rts
hplot:  lda COLOR
        jsr plot
        rts
herase: lda #0
        jsr plot
        rts
hcol:   inc COLOR
        lda COLOR
        cmp #4
        bcc hdone
        lda #1
        sta COLOR
        rts
hclr:   jsr fclear
        rts
hsave:  jsr fbcopy
        rts
hload:  jsr fbload
        rts

; --- fp_addr: FP:FPH = $4000 + CURY*32 + CURX/4 -----------------------
fp_addr:
        lda CURY
        sta FP          ; seed the 16-bit value CURY, then *32 below
        lda #0
        sta FPH
        ldy #5
fa_l:   asl FP
        rol FPH
        dey
        bne fa_l
        lda CURX
        lsr a
        lsr a
        clc
        adc FP
        sta FP
        bcc fa_h
        inc FPH
fa_h:   lda FPH
        clc
        adc #$40
        sta FPH
        rts

; --- plot: write color A (0-3) into the fat pixel at (CURX,CURY) ------
plot:   pha
        jsr fp_addr
        pla
        and #3
        sta T1
        ldx CURX
        txa
        and #3
        tax
        lda SH,x
        sta T0
        lda KM,x
        sta T2
        lda T1
        ldx T0
        beq pl_d
pl_l:   asl a
        dex
        bne pl_l
pl_d:   sta T1
        ldy #0
        lda (FP),y
        and T2
        ora T1
        sta (FP),y
        rts

; --- stamp: save the fb byte under the cursor, then stamp it to 3 -----
stamp:  jsr fp_addr
        ldy #0
        lda (FP),y
        sta CURSAV
        lda #3
        jsr plot
        rts

; --- unstamp: restore the saved byte back over the cursor --------------
unstamp:
        jsr fp_addr
        ldy #0
        lda CURSAV
        sta (FP),y
        rts

; --- fclear: zero the 6K framebuffer, one frame ------------------------
fclear: lda #0
        ldx #0
fc_l:   sta $4000,x
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
        bne fc_l
        rts

; --- fbcopy: framebuffer -> $6000 shadow, one unrolled pass ------------
fbcopy: ldx #0
bcp_l:  lda $4000,x
        sta $6000,x
        lda $4100,x
        sta $6100,x
        lda $4200,x
        sta $6200,x
        lda $4300,x
        sta $6300,x
        lda $4400,x
        sta $6400,x
        lda $4500,x
        sta $6500,x
        lda $4600,x
        sta $6600,x
        lda $4700,x
        sta $6700,x
        lda $4800,x
        sta $6800,x
        lda $4900,x
        sta $6900,x
        lda $4A00,x
        sta $6A00,x
        lda $4B00,x
        sta $6B00,x
        lda $4C00,x
        sta $6C00,x
        lda $4D00,x
        sta $6D00,x
        lda $4E00,x
        sta $6E00,x
        lda $4F00,x
        sta $6F00,x
        lda $5000,x
        sta $7000,x
        lda $5100,x
        sta $7100,x
        lda $5200,x
        sta $7200,x
        lda $5300,x
        sta $7300,x
        lda $5400,x
        sta $7400,x
        lda $5500,x
        sta $7500,x
        lda $5600,x
        sta $7600,x
        lda $5700,x
        sta $7700,x
        inx
        bne bcp_j
        rts
bcp_j:  jmp bcp_l

; --- fbload: $6000 shadow -> framebuffer, same pass reversed -----------
fbload: ldx #0
bl_l:   lda $6000,x
        sta $4000,x
        lda $6100,x
        sta $4100,x
        lda $6200,x
        sta $4200,x
        lda $6300,x
        sta $4300,x
        lda $6400,x
        sta $4400,x
        lda $6500,x
        sta $4500,x
        lda $6600,x
        sta $4600,x
        lda $6700,x
        sta $4700,x
        lda $6800,x
        sta $4800,x
        lda $6900,x
        sta $4900,x
        lda $6A00,x
        sta $4A00,x
        lda $6B00,x
        sta $4B00,x
        lda $6C00,x
        sta $4C00,x
        lda $6D00,x
        sta $4D00,x
        lda $6E00,x
        sta $4E00,x
        lda $6F00,x
        sta $4F00,x
        lda $7000,x
        sta $5000,x
        lda $7100,x
        sta $5100,x
        lda $7200,x
        sta $5200,x
        lda $7300,x
        sta $5300,x
        lda $7400,x
        sta $5400,x
        lda $7500,x
        sta $5500,x
        lda $7600,x
        sta $5600,x
        lda $7700,x
        sta $5700,x
        inx
        bne bl_j
        rts
bl_j:   jmp bl_l

; --- tables -------------------------------------------------------------
SH:     .byte 6,4,2,0
KM:     .byte $3F,$CF,$F3,$FC

stub:   rti

        .org $FFFA
        .word stub,start,stub