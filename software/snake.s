; VINTAGE-1
; Author: roywalk3r
; Repo: https://github.com/roywalk3r/vintage
; License: MIT
; Snake — arrows at $5800 ($11 up, $12 down, $13 left, $14 right),
; +/- (0x15/0x16) retune the move divider live (1 = fastest, 8 = slowest).
; The framebuffer is 32 bytes wide, so grid cell (x,y) lives at fb + y*256 + x:
; one byte per scanline, one cell = 8 scanlines tall. Cells are updated
; incrementally: erase tail, stamp head — a full clear won't fit in a frame.

FB   = $4000
KEY  = $5800
RND  = $5805

; --- zero page ---
DIR    = $E0      ; 0=right 1=down 2=left 3=up
NEWDIR = $E1
LEN    = $E2
HX     = $E3
HY     = $E4
FX     = $E5
FY     = $E6
CNT    = $E7
FCTR   = $E8
DLO    = $E9
DHI    = $EA
TX     = $EB
TY     = $EC
TFLAG  = $ED
VAL    = $EE
CLX    = $EF
CLY    = $F0
SPEED  = $F1      ; step every SPEED frames (1..8)

; --- body arrays in low RAM ---
SX     = $6100
SY     = $6140

 .org $E000

; --- init: 3-segment snake mid-left heading right, full screen draw ---
start:
 lda #6
 sta SX
 lda #5
 sta SX+1
 lda #4
 sta SX+2
 lda #12
 sta SY
 sta SY+1
 sta SY+2
 lda #3
 sta LEN
 lda #0
 sta DIR
 sta NEWDIR
 sta CNT
 sta FCTR
 lda #4
 sta SPEED
 jsr redraw_all

; --- main loop: wait for the frame tick, then update ---
main:
 lda $5802
 cmp FCTR
 beq main
 sta FCTR
 jsr readkey
 jsr maybe_step
 jmp main

; --- keyboard: newest key wins, read-clears ---
readkey:
 lda KEY
 beq rkdone
 cmp #$11
 bne rk2
 lda #3
 sta NEWDIR
rkdone: rts
rk2:
 cmp #$12
 bne rk3
 lda #1
 sta NEWDIR
 rts
rk3:
 cmp #$13
 bne rk4
 lda #2
 sta NEWDIR
 rts
rk4:
 cmp #$14
 bne rk5
 lda #0
 sta NEWDIR
 rts
rk5:
 cmp #$2B        ; '+' = faster (ASCII, so basic/calc share the keymap)
 bne rk6
 dec SPEED
 bne rk4b
 inc SPEED       ; clamp at 1 = fastest
rk4b:
 rts
rk6:
 cmp #$2D        ; '-' = slower
 bne rkdone
 inc SPEED
 cmp #8
 bcc rk6b
 lda #$08
 sta SPEED
rk6b:
 rts

; --- step if the move divider says so (every SPEED frames) ---
maybe_step:
 inc CNT
 lda CNT
 cmp SPEED
 bcc ms1
 lda #0
 sta CNT
 jsr step
ms1: rts

; --- advance the snake one cell, then patch just the changed pixels ---
step:
 ; remember the tail (erased unless the snake grows this move)
 ldx LEN
 dex
 lda SX,x
 sta TX
 lda SY,x
 sta TY
 lda #1
 sta TFLAG
 ; apply pending direction unless it reverses 180 degrees
 lda NEWDIR
 cmp DIR
 beq st1
 lda DIR
 clc
 adc #2
 and #3
 cmp NEWDIR
 beq st1
 lda NEWDIR
 sta DIR
st1:
 ; new head = head + delta
 ldx DIR
 lda DXT,x
 clc
 adc SX
 sta HX
 lda DYT,x
 clc
 adc SY
 sta HY
 ; wall death: the head dies ON the boundary cell, never over it;
 ; 8-bit wrap turns -1 into $FF, caught by the bcs
 lda HX
 beq d2
 cmp #31
 bcs d2
 lda HY
 beq d2
 cmp #23
 bcs d2
 ; self collision: any body cell (except the head itself) at (HX,HY)?
 ldx LEN
 dex
sc1: lda SX,x
 cmp HX
 bne sc2
 lda SY,x
 cmp HY
 beq d2
sc2: dex
 bne sc1
 jmp st1f
d2: jmp crash
 ; ate the food? cap at 63 segments
st1f:
 lda HX
 cmp FX
 bne st1c
 lda HY
 cmp FY
 bne st1c
 lda LEN
 cmp #63
 bcs st1b
 inc LEN
st1b:
 lda #0
 sta TFLAG
 jsr place_food
st1c:
 ; shift body: SX[i] = SX[i-1] for i = LEN-1 .. 1
 ldx LEN
 dex
sh1: lda SX-1,x
 sta SX,x
 lda SY-1,x
 sta SY,x
 dex
 bne sh1
 ; head goes to index 0
 lda HX
 sta SX
 lda HY
 sta SY
 ; erase the old tail cell
 lda TFLAG
 beq nr1
 lda TX
 sta CLX
 lda TY
 sta CLY
 lda #0
 jsr fillcell
nr1:
 ; stamp the new head cell
 lda SX
 sta CLX
 lda SY
 sta CLY
 lda #$FF
 jsr fillcell
 rts

crash:
 jmp start

; --- full redraw (init / game restart only) ---
redraw_all:
 ; clear: fb = 24 pages exactly
 lda #0
 ldx #0
cloop:
 sta FB,x
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
 bne cloop
 ; top and bottom walls: fb pages $4000 and $5700
 lda #$FF
 ldx #0
bt1: sta FB,x
 inx
 bne bt1
bt2: sta FB+$1700,x
 inx
 bne bt2
 ; side walls: cell (0,cy) and (31,cy) for cy=1..22
 lda #0
 sta CLY
sw1: lda #0
 sta CLX
 lda #$FF
 jsr fillcell
 lda #31
 sta CLX
 lda #$FF
 jsr fillcell
 inc CLY
 lda CLY
 cmp #23
 bne sw1
 ; snake body cells
 ldx #0
r1: lda SX,x
 sta CLX
 lda SY,x
 sta CLY
 lda #$FF
 jsr fillcell
 inx
 cpx LEN
 bne r1
 ; food
 jsr place_food
 rts

; --- fill the 8x8 cell at (CLX,CLY) with the value in A ---
fillcell:
 sta VAL
 lda CLX
 sta DLO
 lda #$40
 clc
 adc CLY
 sta DHI
 lda VAL
 ldy #0
fc1: lda VAL
 sta (DLO),y
 tya
 clc
 adc #32
 tay
 bne fc1
 rts

; --- 4x4 food dot centred in the cell at (FX,FY) ---
draw_food:
 lda FX
 sta DLO
 lda #$40
 clc
 adc FY
 sta DHI
 ldy #0
 lda #0
 sta (DLO),y
 ldy #32
 sta (DLO),y
 ldy #64
 lda #$3C
 sta (DLO),y
 ldy #96
 sta (DLO),y
 ldy #128
 sta (DLO),y
 ldy #160
 sta (DLO),y
 ldy #192
 lda #0
 sta (DLO),y
 ldy #224
 sta (DLO),y
 rts

; --- random food spot, interior cells only (walls excluded) ---
place_food:
 lda RND
 and #31
 cmp #1
 bcc place_food    ; retry on wall x=0
 cmp #31
 bcs place_food    ; retry on wall x=31 / wrapped
 sta FX
pfy:
 lda RND
 and #31
 cmp #1
 bcc pfy           ; 0 -> shift into range
 cmp #23
 bcs pfy           ; 23..31 -> retry
 sta FY
 jsr draw_food
 rts

; --- direction delta tables ---
DXT: .byte 1,0,$FF,0
DYT: .byte 0,1,0,$FF

 .org $FFFC
 .word start