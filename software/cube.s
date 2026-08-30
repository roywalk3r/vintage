; VINTAGE-1
; Author: roywalk3r
; Repo: https://github.com/roywalk3r/vintage
; License: MIT
; VINTAGE-1 rotating wireframe cube — software 3-D, no FPU, no tables but
; one: the 32-step projected vertex table is precomputed (cube_table.py),
; 16 bytes per step (8 vertices x px,py). Every frame-with-cube-moved has
; to clear and redraw within its frame budget, so the clear only wipes the
; cube's bounding box and 12 Bresenham edges get redrawn; math stays in
; signed 8-bit range because edge spans stay under 64 px.

 .org $E000

FB  = $4000
FC  = $5802

; --- zero page ---
FCTR = $E8
CNT  = $E7
STEP = $E6
TLO  = $E9
THI  = $EA
X0   = $EB
X1   = $EC
Y0   = $ED
Y1   = $EE
DXA  = $EF
DYA  = $F1
SXF  = $F2
ERR  = $F4
TMP  = $F5
TMP2 = $FB
PLO  = $F7
PHI  = $F8
CY   = $F9
EI   = $FA

start:
 lda #0
 sta FCTR
 sta CNT
 sta STEP
 ; clear whole screen once (32k cycles — only at boot)
 lda #0
 ldx #0
c1: sta FB,x
 sta FB+$100,x
 sta FB+$200,x
 sta FB+$300,x
 sta FB+$400,x
 sta FB+$500,x
 sta FB+$600,x
 sta FB+$700,x
 sta FB+$800,x
 sta FB+$900,x
 sta FB+$A00,x
 sta FB+$B00,x
 sta FB+$C00,x
 sta FB+$D00,x
 sta FB+$E00,x
 sta FB+$F00,x
 sta FB+$1000,x
 sta FB+$1100,x
 sta FB+$1200,x
 sta FB+$1300,x
 sta FB+$1400,x
 sta FB+$1500,x
 sta FB+$1600,x
 sta FB+$1700,x
 inx
 bne c1
main:
 ; wait for a frame tick
 lda $5802
 cmp FCTR
 beq main
 sta FCTR
 inc CNT
 lda CNT
 and #3
 bne main          ; draw every 4th frame
 ; --- cube bounding box clear (cell rows 9-14, x bytes 12-19) ---
 lda #10
 sta CY
cb1:
 lda #13
 sta PLO
 lda #$40
 clc
 adc CY
 sta PHI
 ldx #8            ; 8 scanlines per cell row
cb2:
 lda #0
 ldy #5
cb3: sta (PLO),y
 dey
 bpl cb3
 ; advance one scanline (+32)
 lda PLO
 clc
 adc #32
 sta PLO
 bcc cb4
 inc PHI
cb4:
 dex
 bne cb2
 inc CY
 lda CY
 cmp #14
 bne cb1
 ; --- draw cube at current rotation step ---
 ; pointer = TBL + STEP*16
 lda STEP
 asl
 asl
 asl
 asl
 sta TMP
 lda STEP
 lsr
 lsr
 lsr
 lsr
 sta TMP2
 lda #<TBL
 clc
 adc TMP
 sta TLO
 lda #TBL>>8
 adc TMP2
 sta THI
 ; 12 edges
 lda #0
 sta EI
e1:
 ; a = ETAB[2*EI], b = ETAB[2*EI+1]; coords at TBL+2a, TBL+2b
 lda EI
 asl
 tay
 lda ETAB,y
 asl
 tay
 lda (TLO),y
 sta X0
 iny
 lda (TLO),y
 sta Y0
 lda EI
 asl
 clc
 adc #1
 tay
 lda ETAB,y
 asl
 tay
 lda (TLO),y
 sta X1
 iny
 lda (TLO),y
 sta Y1
 jsr line
 inc EI
 lda EI
 cmp #12
 bne e1
 inc STEP
 lda STEP
 and #31
 sta STEP
 jmp main

; --- plot: XOR pixel (X0,Y0) into the framebuffer ---
plot:
 lda Y0
 lsr
 lsr
 lsr
 clc
 adc #$40
 sta PHI
 lda Y0
 and #7
 asl
 asl
 asl
 asl
 asl
 tax
 lda X0
 lsr
 lsr
 lsr
 sta TMP
 txa
 clc
 adc TMP
 sta PLO
 lda X0
 and #7
 tay
 lda BITMASK,y
 ldy #0
 eor (PLO),y
 sta (PLO),y
 rts

; --- Bresenham line (X0,Y0) -> (X1,Y1), XOR plot ---
; order endpoints by y, split on dominant axis; the draw loops plot every
; pixel exactly once.
line:
 ; order by y: if Y1 < Y0, swap endpoints
 lda Y1
 cmp Y0
 bcs lo1
 lda X0
 sta TMP
 lda X1
 sta X0
 lda TMP
 sta X1
 lda Y0
 sta TMP
 lda Y1
 sta Y0
 lda TMP
 sta Y1
lo1:
 ; dx signed + abs, sx = +1/$FF
 lda X1
 sec
 sbc X0
 sta DXA
 bpl dx1
 lda #$FF
 sta SXF
 lda DXA
 eor #$FF
 clc
 adc #1
 sta DXA
 jmp dx3
dx1:
 lda #$01
 sta SXF
dx3:
 ; dy = Y1-Y0 (>= 0 after the swap)
 lda Y1
 sec
 sbc Y0
 sta DYA
 ; dispatch on dominant axis
 lda DXA
 cmp DYA
 bcc liny
 ; --- x-dominant: err = dx/2 ---
 lda DXA
 lsr
 sta ERR
lx1:
 jsr plot
 lda ERR
 sec
 sbc DYA
 sta ERR
 bcs lx2
 clc
 adc DXA
 sta ERR
 inc Y0
lx2:
 lda X0
 clc
 adc SXF
 sta X0
 cmp X1
 bne lx1
 rts

liny:
 lda DYA
 lsr
 sta ERR
ly1:
 jsr plot
 lda ERR
 sec
 sbc DXA
 sta ERR
 bcs ly2
 clc
 adc DYA
 sta ERR
 lda X0
 clc
 adc SXF
 sta X0
ly2:
 inc Y0
 lda Y0
 cmp Y1
 bne ly1
 rts

BITMASK: .byte $80,$40,$20,$10,$08,$04,$02,$01

; 12 edges as vertex-index pairs
ETAB:
 .byte 0,1, 1,3, 3,2, 2,0
 .byte 4,5, 5,7, 7,6, 6,4
 .byte 0,4, 1,5, 2,6, 3,7

TBL:
; bbox x 114..142  y 82..110
 .byte $74,$6C,$8C,$6C,$74,$54,$8C,$54,$78,$68,$88,$68,$78,$58,$88,$58 ; step 0
 .byte $72,$6C,$8A,$6D,$72,$54,$8A,$53,$7A,$67,$89,$68,$7A,$59,$89,$58
 .byte $72,$6B,$87,$6E,$72,$55,$87,$52,$7C,$67,$8B,$68,$7C,$59,$8B,$58
 .byte $72,$6A,$84,$6E,$72,$56,$84,$52,$7E,$67,$8C,$69,$7E,$59,$8C,$57
 .byte $73,$6A,$80,$6E,$73,$56,$80,$52,$80,$67,$8D,$6A,$80,$59,$8D,$56
 .byte $74,$69,$7C,$6E,$74,$57,$7C,$52,$82,$67,$8E,$6A,$82,$59,$8E,$56
 .byte $75,$68,$79,$6E,$75,$58,$79,$52,$84,$67,$8E,$6B,$84,$59,$8E,$55
 .byte $77,$68,$76,$6D,$77,$58,$76,$53,$86,$67,$8E,$6C,$86,$59,$8E,$54
 .byte $78,$68,$74,$6C,$78,$58,$74,$54,$88,$68,$8C,$6C,$88,$58,$8C,$54 ; step 8
 .byte $7A,$67,$72,$6C,$7A,$59,$72,$54,$89,$68,$8A,$6D,$89,$58,$8A,$53
 .byte $7C,$67,$72,$6B,$7C,$59,$72,$55,$8B,$68,$87,$6E,$8B,$58,$87,$52
 .byte $7E,$67,$72,$6A,$7E,$59,$72,$56,$8C,$69,$84,$6E,$8C,$57,$84,$52
 .byte $80,$67,$73,$6A,$80,$59,$73,$56,$8D,$6A,$80,$6E,$8D,$56,$80,$52
 .byte $82,$67,$74,$69,$82,$59,$74,$57,$8E,$6A,$7C,$6E,$8E,$56,$7C,$52
 .byte $84,$67,$75,$68,$84,$59,$75,$58,$8E,$6B,$79,$6E,$8E,$55,$79,$52
 .byte $86,$67,$77,$68,$86,$59,$77,$58,$8E,$6C,$76,$6D,$8E,$54,$76,$53
 .byte $88,$68,$78,$68,$88,$58,$78,$58,$8D,$6C,$74,$6C,$8D,$54,$74,$54 ; step 16
 .byte $89,$68,$7A,$67,$89,$58,$7A,$59,$8A,$6D,$72,$6C,$8A,$53,$72,$54
 .byte $8B,$68,$7C,$67,$8B,$58,$7C,$59,$87,$6E,$72,$6B,$87,$52,$72,$55
 .byte $8C,$69,$7E,$67,$8C,$57,$7E,$59,$84,$6E,$72,$6A,$84,$52,$72,$56
 .byte $8D,$6A,$80,$67,$8D,$56,$80,$59,$80,$6E,$73,$6A,$80,$52,$73,$56
 .byte $8E,$6A,$82,$67,$8E,$56,$82,$59,$7C,$6E,$74,$69,$7C,$52,$74,$57
 .byte $8E,$6B,$84,$67,$8E,$55,$84,$59,$79,$6E,$75,$68,$79,$52,$75,$58
 .byte $8E,$6C,$86,$67,$8E,$54,$86,$59,$76,$6D,$77,$68,$76,$53,$77,$58
 .byte $8D,$6C,$88,$68,$8D,$54,$88,$58,$74,$6C,$78,$68,$74,$54,$78,$58 ; step 24
 .byte $8A,$6D,$89,$68,$8A,$53,$89,$58,$72,$6C,$7A,$67,$72,$54,$7A,$59
 .byte $87,$6E,$8B,$68,$87,$52,$8B,$58,$72,$6B,$7C,$67,$72,$55,$7C,$59
 .byte $84,$6E,$8C,$69,$84,$52,$8C,$57,$72,$6A,$7E,$67,$72,$56,$7E,$59
 .byte $80,$6E,$8D,$6A,$80,$52,$8D,$56,$73,$6A,$80,$67,$73,$56,$80,$59
 .byte $7C,$6E,$8E,$6A,$7C,$52,$8E,$56,$74,$69,$82,$67,$74,$57,$82,$59
 .byte $79,$6E,$8E,$6B,$79,$52,$8E,$55,$75,$68,$84,$67,$75,$58,$84,$59
 .byte $76,$6D,$8E,$6C,$76,$53,$8E,$54,$77,$68,$86,$67,$77,$58,$86,$59

 .org $FFFC
 .word start
