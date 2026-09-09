; VINTAGE-1
; Author: roywalk3r
; Repo: https://github.com/roywalk3r/vintage
; License: MIT
; basic.s - line-numbered tiny BASIC: FOR/NEXT (with signed STEP), INPUT,
; LET/PRINT/GOTO/IF...GOTO/POKE as one-statement lines in a 32-slot
; program store, plus direct RUN, LIST, NEW, and immediate versions of
; most statements. Expressions evaluate 16-bit over variables A-Z with
; * / binding tighter than + -, parentheses, unary minus, and RND /
; PEEK(addr) as primaries. The screen is an 8-row scrolling terminal
; (rows 0-7) with the input line on row 8, and every printed row is
; mirrored as ASCII at $2500 for headless tests.
;
; Program store: 32 slots of 32 bytes at $2000, [LNLO, LNHI, LEN,
; TEXT...], bump-allocated by shifting the sorted tail. Direct commands
; reuse the same statement executor by pointing CPTR at the input
; buffer, so mode is just "which RAM the text lives in".

        .org $E000
SCREEN = $4000
IBUF   = $1000         ; input line, NUL-terminated, 28 chars max
VARS   = $1100         ; A-Z, 2 bytes each
PROG   = $2000         ; 32 slots x 32 bytes: [LNLO, LNHI, LEN, text...]
TERM   = $2500         ; terminal mirror, 8 rows x (32 chars + NUL)
IBUFM  = $2600         ; input-line mirror (33 bytes: prompt + text + pad)
DBUF   = $2700         ; decimal digit scratch, LSB-first
TB     = $2740         ; 33-byte compose buffer for PRINT/LIST lines
STRV   = $1200         ; A$-Z$, 8 bytes each: 7 chars + NUL, NUL-padded
SSCR   = $5C           ; string build buffer, 8 bytes in zero page
SLEN   = $64           ; composed string length (0..7)
SPTR   = $65           ; string-var slot pointer for (zp),y access
SPTRH  = $66
TBLEN  = $67           ; PRINT compose cursor into TB
SINF   = $68           ; INPUT is targeting a string variable
RDLO   = $69           ; DATA/READ data cursor: slot base = scanning,
RDHI   = $6A           ; otherwise mid-list inside a DATA line's items
PX      = $6B           ; PLOT x (0..255), y (0..191), mode: 1 set 0 clear
PLY     = $6C
XPLOTF  = $6D

; --- zero page ---
IBLEN  = $12           ; input buffer length
TK     = $13
CPTR   = $14           ; execution position: slot base (run) or $100x (direct)
CPTRH  = $15
OUTROW = $16           ; next terminal output row 0..7
ERRF   = $17           ; set by xerr: aborts the run
; --- 16-bit math workspace, the calc.s layout ---
DVND   = $18
DVNDH  = $19
M2     = $1A
M2H    = $1B
M1     = $1C
M1H    = $1D
RES    = $1E
RESH   = $1F
REM    = $20
REMH   = $21
QUO    = $22
QUOH   = $23
T0     = $24
T0H    = $25
T1     = $26
T1H    = $27
; --- expression state ---
ACC    = $28
ACCH   = $29
RHS    = $2A
RHSH   = $2B
PEND   = $2C          ; pending op 0..4 (0 none, 1 +, 2 -, 3 *, 4 /)
VIDX   = $2D          ; var slot of the LET target
DGTF   = $2E          ; pnum: saw a digit this call
COMP   = $2F          ; IF comparison op 1 '=' 2 '<' 3 '>'
NUMPROG = $30
PROGTOP = $31         ; slot address one past the last line (hi byte only:
PROGEH  = $32         ; 32 slots x 32 bytes keeps the store under one page)
TIDX   = $33          ; LIST: slot index scratch
LN     = $34
LNH    = $35
LEN    = $36          ; xstore: parked insert index during the shift
STXV   = $3E          ; xstore: IBUF index of the line text
FSP    = $3F          ; FOR/NEXT: loop levels in use (0..4)
FORST  = $40          ; 4 levels x 7 bytes: [var, limit lo/hi, step lo/hi,
                      ; ret lo/hi]; ret = CPTR+32 at FOR time
DISPL  = $37          ; to_dec scratch (calc port, re-pointed)
DISPH  = $38
DLEN   = $39
SRC    = $3A          ; scratch pointers (rowptr, xstore, shift)
SRCH   = $3B
DST    = $3C
DSTH   = $3D
MSGLO = $E0
MSGHI = $E1
DLO   = $E2
DHI   = $E3
FLO   = $E4          ; font pointer: never shares a cell with DLO/DHI
FHI   = $E5
GLYPH = $E6
GHI    = $E7
CHIDX = $E8
PBUF   = $2780        ; input-row compose buffer (33 bytes)
GSTK   = $2721        ; GOSUB frames: 4 x [ret lo @+0, ret hi @+4, FSP @+8]
GSP    = $272D        ; live GOSUB depth, 0..4

; --- boot: clear everything, print READY, then the poll loop ------------
start:  jsr clear_scr
        jsr tclear
        jsr szero
        lda #0
        sta OUTROW
        sta IBLEN
        sta NUMPROG
        sta PROGTOP      ; PROGTOP = $2000 (no lines)
        lda #PROG/$100
        sta PROGEH
        jsr rready
        jsr rprompt

poll:   lda $5800
        beq poll
        jsr handle
        jsr rprompt      ; re-render the input row after every key
        jmp poll

; --- key dispatch: A = the raw $5800 byte -------------------------------
handle: sta TK
        cmp #$0D
        beq hsub_j
        cmp #$08
        beq hbksp
        cmp #$20
        bcc hdone
        cmp #$7F
        bcc happend_j
hdone:  rts

; branch-range trampolines
hsub_j: jmp hsubmit
happend_j: jmp happend
hbksp:  lda IBLEN
        beq hdone       ; empty: ignore
        dec IBLEN
        ldy IBLEN
        lda #0
        sta IBUF,y      ; keep IBUF NUL-terminated for the parsers
        rts

happend:
        ldx IBLEN
        cpx #28
        bcs hdone       ; buffer full: ignore
        sta IBUF,x
        inx
        lda #0
        sta IBUF,x      ; terminator tracks the length
        stx IBLEN
        rts
hsubmit:
        lda IBLEN
        beq hdone        ; empty line: nothing to do
        lda #<IBUF
        sta CPTR
        lda #IBUF/$100
        sta CPTRH
        ldy #0
        jsr pnum         ; ACC = line number if digits were typed
        jsr skipsp
        lda DGTF
        beq dexec        ; no digits: direct command
        ; program line: LN in ACC, text at (CPTR),y
        lda ACC
        sta LN
        lda ACCH
        sta LNH
        lda (CPTR),y
        bne hstor_j      ; text present: insert/replace
        jmp hbare        ; bare line number: delete
hbare:  jsr dline
        jmp hcln
hstor_j: jsr xstore
        jmp hcln
drr_j:  jmp drun
dnw_j:  jmp xnew
xlj_j:  jsr xlist
        jmp hcln         ; xlist rts'd past hcln, leaving the line buffered
dexec:  lda IBUF
        cmp #'R'
        bne d1
        lda IBUF+1
        cmp #'E'         ; direct RETURN = RETURN without GOSUB: ERR
        beq dxr
        jmp drr_j        ; RUN
dxr:    jmp xerr
d1:     cmp #'N'
        beq dnew
        cmp #'E'         ; END direct: retire the line, not just the flag
        beq dhcln_j
        cmp #'L'
        bne d2
        lda IBUF+1
        cmp #'E'
        bne xlj_j
        ; direct LET: the LET handler parses from (CPTR),y=0
        ldy #0
        jsr xlet
        jmp hcln
d2:     cmp #'P'
        beq dpk
        cmp #'G'
        bne d3
        lda IBUF+2
        cmp #'S'         ; direct GOSUB: ERR (no direct-mode return stack)
        beq dxg
        jmp dgoto_j
dxg:    jmp xerr
dupl_j: jmp dupl         ; branch trampoline (dead cell: dxg always jumps)
d3:     cmp #'I'
        beq difi
        cmp #'U'
        beq dupl_j       ; direct UNPLOT
        jmp xerr         ; unknown direct command
dnew:   lda IBUF+2
        cmp #'X'
        beq dnx_j        ; NEXT
        jmp dnw_j        ; NEW
dnx_j:  lda #<IBUF
        sta CPTR
        lda #IBUF/$100
        sta CPTRH
        ldy #0
        jsr xnext
        jmp hcln
dpk:    lda IBUF+1
        cmp #'O'
        beq dpk_j        ; POKE
        cmp #'L'
        beq dpl_j        ; PLOT
        jmp dprint       ; PRINT
dpk_j:  lda #<IBUF
        sta CPTR
        lda #IBUF/$100
        sta CPTRH
        ldy #0
        jsr xpoke
        jmp hcln
difi:   lda IBUF+1
        cmp #'N'
        beq din_j        ; INPUT
        jmp dif_d        ; IF
dhcln_j:
        jsr hcln
        rts              ; direct-mode exit that must retire the line
din_j:  lda #<IBUF
        sta CPTR
        lda #IBUF/$100
        sta CPTRH
        ldy #0
        jsr xinput
        jmp hcln
dprint:
        lda #<IBUF
        sta CPTR
        lda #IBUF/$100
        sta CPTRH
        ldy #0
        jsr xprint
        jmp hcln
dpl_j:  jmp dplf         ; far: direct PLOT (E000 is full)
dupl:   jmp dupf         ; far: direct UNPLOT
dgc_j:  jmp hcln
dgoto_j:
        lda #<IBUF
        sta CPTR
        lda #IBUF/$100
        sta CPTRH
        ldy #0
        jsr xgoto
        lda ERRF
        bne dgc_j
        lda #0
        sta FSP          ; fresh FOR stack for the direct-GOTO run
        jsr rdinit       ; fresh data cursor for the direct-GOTO run
        jsr xloop
        jmp hcln         ; found: run from the target line, then clear
dif_d:
        lda #<IBUF
        sta CPTR
        lda #IBUF/$100
        sta CPTRH
        ldy #0
        jsr xif
        bcs dif1         ; taken: CPTR is on the target slot
        jmp hcln         ; false: stay in direct mode
dif1:   lda #0
        sta FSP          ; fresh FOR stack for the direct-IF run
        jsr rdinit       ; fresh data cursor for the direct-IF run
        jsr xloop
        jmp hcln         ; xloop rts'd past hcln (same leak as LIST)
; --- terminal helper: row pointers --------------------------------------
; A = row 0..7 -> SRC/SRCH = TERM + 33*row (offset <= 231, no carry)
rowptr: stx T0H        ; tclear/tscroll count rows in X: preserve it
        sta T0         ; row; A is untouched from here to the adc
        ldy #5
rp1r:   asl T0
        dey
        bne rp1r         ; T0 = 32*row
        clc
        adc T0           ; 33*row, fits a byte
        sta SRC
        lda #<TERM
        clc
        adc SRC
        sta SRC
        lda #TERM/$100
        sta SRCH
        ldx T0H
        rts

; A = row: space-fill the mirror row, NUL at +32
rowfill:
        jsr rowptr
        ldy #32
        lda #0
        sta (SRC),y
        ldy #31
rf1:    lda #$20
        sta (SRC),y
        dey
        bpl rf1
        rts
tclear: ldx #0
tl0:    txa
        jsr rowfill
        inx
        cpx #8
        bcc tl0
        rts

; text at MSGLO/HI -> printed on the next output row, scrolling if needed
tprint: lda OUTROW
        cmp #8
        bcc tp1
        jsr tscroll
        lda #7
        sta OUTROW
tp1:    lda OUTROW
        jsr rowfill
        ldy #0
tpc:    lda (MSGLO),y
        beq tpdone
        sta (SRC),y
        iny
        cpy #32
        bcc tpc
tpdone:
        lda SRC
        sta MSGLO
        lda SRCH
        sta MSGHI
        lda #0
        sta DLO
        lda OUTROW
        clc
        adc #$40
        sta DHI
        jsr draw_msg
        inc OUTROW
        rts

; --- scroll: mirror rows 1..7 up one row, then the 8 framebuffer pages --
tscroll:
        ldx #0
ts1:    txa
        jsr rowptr       ; SRC = row x
        lda SRC
        clc
        adc #33
        sta DST
        lda SRCH
        adc #0
        sta DSTH         ; DST = row x+1
        ldy #0
ts2:    lda (DST),y
        sta (SRC),y
        iny
        cpy #33
        bcc ts2
        inx
        cpx #7
        bcc ts1
        lda #7
        jsr rowfill
        ldx #0
tsf1:   lda $4100,x
        sta $4000,x
        lda $4200,x
        sta $4100,x
        lda $4300,x
        sta $4200,x
        lda $4400,x
        sta $4300,x
        lda $4500,x
        sta $4400,x
        lda $4600,x
        sta $4500,x
        lda $4700,x
        sta $4600,x
        lda #0
tsf2:   sta $4700,x
        inx
        bne tsf2
        rts
; --- rready / rprompt: boot banner and the input row --------------------
; rprompt composes the '?' + text prompt into the 33-byte row at IBUFM,
; leaves it mirrored there for headless tests, and blits it to row 8.
rready: lda #<readymsg
        sta MSGLO
        lda #readymsg/$100
        sta MSGHI
        jsr tprint
        lda #<readymgs2
        sta MSGLO
        lda #readymgs2/$100
        sta MSGHI
        jsr tprint
        rts

rprompt:
        lda #'?'
        sta IBUFM
        lda #' '
        sta IBUFM+1
        ldx #0
rp1:    cpx IBLEN
        bcs rp2
        lda IBUF,x
        sta IBUFM+2,x
        inx
        jmp rp1
rp2:    lda #$20
rp3:    sta IBUFM+2,x
        inx
        cpx #30
        bcc rp3
        lda #0
        sta IBUFM+2,x
        lda #<IBUFM
        sta MSGLO
        lda #IBUFM/$100
        sta MSGHI
        lda #0
        sta DLO
        lda #$48
        sta DHI
        jsr draw_msg
        rts

; --- hcln: empty the input line -----------------------------------------
hcln:   lda #0
        sta IBLEN
        sta IBUF
        sta ERRF        ; a finished direct command/run retires the abort
        rts             ; flag, so a past error can't no-op later runs
; --- pnum: decimal digit run at (CPTR),y -> ACC --------------------------
; Sets DGTF when at least one digit was consumed; leaves the first
; non-digit unconsumed. Values wrap modulo 65536 like every other op.
pnum:   lda #0
        sta DGTF
        sta ACC
        sta ACCH
pn1:    lda (CPTR),y
        cmp #'0'
        bcc pn2
        cmp #':'
        bcs pn2
        sec
        sbc #$30
        sta TK           ; digit, parked while ACC*10 is computed
        lda ACC
        sta T1
        lda ACCH
        sta T1H
        asl T1
        rol T1H          ; T1 = 2*ACC
        lda T1
        sta T0
        lda T1H
        sta T0H          ; T0 = 2*ACC
        asl T1
        rol T1H          ; 4*ACC
        asl T1
        rol T1H          ; 8*ACC
        lda T1
        clc
        adc T0
        sta T0           ; T0 = 10*ACC
        lda T1H
        adc T0H
        sta T0H
        lda TK
        clc
        adc T0
        sta T0
        lda #0
        adc T0H
        sta T0H
        lda T0
        sta ACC
        lda T0H
        sta ACCH
        lda #1
        sta DGTF
        iny
        jmp pn1
pn2:    rts
; --- skipsp / factor / expr ----------------------------------------------
skipsp: lda (CPTR),y
        cmp #' '
        bne sk1
        iny
        jmp skipsp
sk1:    rts

; factor: skipsp, then a number (digit run via pnum), a variable A-Z, RND,
; PEEK(expr), a parenthesized expression, or a unary minus. Anything else
; yields 0. ACC holds the 16-bit result; y advances past what was consumed.
factor: jsr skipsp
        lda #0
        sta ACC
        sta ACCH
        lda (CPTR),y
        cmp #'0'
        bcc fac1
        cmp #':'
        bcs fac1
        jmp pnum         ; digit run: pnum leaves ACC and y set
fac1:   cmp #$41
        bcc fac5         ; below 'A': parens, minus, or nothing
        cmp #$5B
        bcs fac5
        sta T1           ; park the letter: T1 is factor-local scratch
        iny
        lda (CPTR),y
        cmp #'E'
        bne fac3
        lda T1
        cmp #'P'
        beq fpeek        ; "PE..." -> PEEK
        cmp #'L'
        beq fle_j        ; "LE..." -> LEN (trampoline: flen is far)
fac3:   cmp #'N'
        bne fac4
        lda T1
        cmp #'R'
        beq frnd         ; "RN..." -> RND
fac4:   lda T1           ; plain variable: y is already past the letter
        sec
        sbc #$41
        asl a
        tax              ; VIDX = 2*(c-A)
        sta VIDX
        lda VARS,x
        sta ACC
        lda VARS+1,x
        sta ACCH
        rts
fac5:   cmp #'('
        bne fac6
        iny
        jsr expr         ; nested parse: expr/term park on the hw stack
        jsr skipsp
        lda (CPTR),y
        cmp #')'
        bne fpar_e
        iny
        rts
fac6:   cmp #'-'
        bne fac7
        iny
        jsr factor
        sec              ; negate: ACC = 0 - ACC (two's complement)
        lda #0
        sbc ACC
        sta T0
        lda #0
        sbc ACCH
        sta ACCH
        lda T0
        sta ACC
        rts
fac7:   rts               ; neither: ACC = 0, unconsumed
fpar_e: jmp xerr          ; malformed PEEK/paren: ERR, abort via ERRF

fle_j:  jmp flen         ; branch-range trampoline (flen lives far away)

frnd:   lda #2
        jsr ady          ; past RND
        lda $5805        ; LFSR read steps it: a fresh byte every call
        sta ACC
        lda #0
        sta ACCH
        rts

fpeek:  lda #3
        jsr ady          ; PEEK is 4 letters and y sits on the 2nd char:
                         ; skip E, E, K to land past the keyword (RND is 3)
        jsr skipsp
        lda (CPTR),y
        cmp #'('
        bne fpar_e
        iny
        jsr expr         ; address expression
        jsr skipsp
        lda (CPTR),y
        cmp #')'
        bne fpar_e
        iny
        lda ACC
        sta T0
        lda ACCH
        sta T0H          ; T0/T0H adjacent: (zp),y pointer
        sty T1
        ldy #0
        lda (T0),y       ; full-bus read: RAM, I/O, ROM all answer
        ldy T1
        sta ACC
        lda #0
        sta ACCH
        rts

; LEN("string expr"): compose the argument through strexpr, then hand
; the composed length back as a 16-bit number so it folds like any
; other factor.
flen:   lda #2
        jsr ady          ; LEN is 3 letters, y sits on the 2nd char:
                         ; skip E, N to land on the '(' (PEEK is 4: +3)
        jsr skipsp
        lda (CPTR),y
        cmp #'('
        bne fpar_e
        iny
        jsr strexpr      ; -> SSCR/SLEN; the numeric running sum is
                         ; parked on the hw stack, y parks in T0
        jsr skipsp
        lda (CPTR),y
        cmp #')'
        bne fpar_e
        iny
        lda SLEN
        sta ACC
        lda #0
        sta ACCH
        rts

; expr: +- level over 16-bit terms; term handles the tighter */ level, so
; 2+3*4 folds as 2+(3*4). Returns with y at the first non-operand char
; (a comparison op, NUL, ...) and ACC = value.
expr:   jsr term
e1:     jsr skipsp
        lda (CPTR),y
        cmp #'+'
        bne e1a
        lda #1
        sta PEND
        jmp e1x
e1a:    cmp #'-'
        bne e1r          ; not an +- op: return, char unconsumed
        lda #2
        sta PEND
e1x:    iny
        lda PEND
        pha              ; term clobbers PEND: save the +- op
        lda ACC
        pha
        lda ACCH
        pha              ; park the running sum on the stack: term clobbers RHS
        jsr term
        pla
        sta RHSH
        pla
        sta RHS
        pla
        sta PEND
        jsr apply
        jmp e1
e1r:    rts

; term: */ level. Same apply machinery as expr but only * and /, so the
; factor result folds into the running product before expr sees it.
term:   jsr factor
t1:     jsr skipsp
        lda (CPTR),y
        cmp #'*'
        bne t1a
        lda #3
        sta PEND
        jmp t1x
t1a:    cmp #'/'
        bne t1r          ; not a */ op: hand control back to expr
        lda #4
        sta PEND
t1x:    iny
        lda ACC
        sta RHS
        lda ACCH
        sta RHSH        ; running product parked for apply
        jsr factor
        jsr apply
        jmp t1
t1r:    rts

; apply: fold factor result (ACC) into the accumulator via PEND;
; M1 = previous ACC, M2 = new factor.
apply:  lda PEND
        beq ap0          ; no pending op: ACC already holds the value
        lda ACC
        sta M2
        lda ACCH
        sta M2H          ; RHS of this step
        jsr apold        ; M1 = the value before this factor
        lda PEND
        cmp #1
        beq apadd
        cmp #2
        beq apsub
        cmp #3
        beq apmul
        jmp apdiv
ap0:    rts
apadd:  lda M2
        clc
        adc M1
        sta ACC
        lda M2H
        adc M1H
        sta ACCH
        rts
apsub:  lda M1
        sec
        sbc M2
        sta ACC
        lda M1H
        sbc M2H
        sta ACCH
        rts
apmul:  jsr mul16
        lda RES
        sta ACC
        lda RESH
        sta ACCH
        rts
apdiv:  lda M2
        bne apd1
        lda M2H
        bne apd1
        jsr xerr         ; divide by zero
        lda #0
        sta ACC
        sta ACCH
        rts
apd1:   lda M1
        sta DVND
        lda M1H
        sta DVNDH
        jsr div16
        lda QUO
        sta ACC
        lda QUOH
        sta ACCH
        rts

; apold: M1 = ACC as it stood before the last factor was parsed.
; factor saves the previous ACC in RHS/RHSH before overwriting it.
apold:  lda RHS
        sta M1
        lda RHSH
        sta M1H
        rts
; --- xstmt: one statement at (CPTR),y. Returns C=1 when the statement
; repositioned CPTR (GOTO, taken IF, END), C=0 to advance 32 bytes ------
xstmt:  ldy #3
        lda (CPTR),y
        cmp #'P'
        bne xs1
        iny
        lda (CPTR),y
        dey              ; peek 2nd char without consuming: handlers
        cmp #'O'         ; expect y at the keyword's first letter
        beq xpk_j
        cmp #'L'
        beq xpl_j        ; PLOT shares PRINT's first letter
        jmp xprint
xpl_j:  jmp xplotf       ; far: PLOT set, UNPLOT clear, shared core
xs1:    cmp #'L'
        bne xs2
        jsr xlet
        clc
        rts
xs2:    cmp #'G'
        bne xs3
        iny
        iny
        lda (CPTR),y     ; 3rd char: S = GOSUB, else GOTO
        dey
        dey
        cmp #'S'
        bne xs2g
        jsr xgosub
        sec
        rts
xs2g:   jsr xgoto
        sec            ; xgoto leaves C set either way (xerr aborts too)
        rts
xs3:    cmp #'I'
        bne xs4
        iny
        lda (CPTR),y
        dey              ; peek 2nd char: IF vs INPUT
        cmp #'N'
        beq xin_j
        jsr xif
        rts            ; xif returns C = condition taken
xs4:    cmp #'E'
        bne xs5
        jmp xend       ; C: see xend
xs5:    cmp #'F'
        bne xs6
        jsr xfor
        rts            ; xfor returns C=0 (fall into the body)
xs6:    cmp #'N'
        bne xs7
        jsr xnext
        rts            ; xnext returns C: 1 resume at the FOR's successor
xs7:    cmp #'R'
        bne xs7d
        iny
        lda (CPTR),y     ; 2nd char: E = the RE... family (3rd char picks)
        dey
        cmp #'E'
        bne xs8
        jmp xr3          ; far dispatch: READ / RETURN / RESTORE, C per handler
xs7d:   cmp #'D'
        bne xs7e
        jmp xd3          ; far: DATA is a no-op statement (data lives here)
xs7e:   cmp #'U'
        bne xs8
        jmp xunplf       ; far: UNPLOT — U has no other keywords yet
xs8:    jmp xerr       ; unknown keyword: ERR, abort
xpk_j:  jsr xpoke
        clc
        rts
xin_j:  jsr xinput
        clc
        rts
; --- xprint: PRINT item ; item ... y at the P of the keyword ------------
; Items compose into TB through a TBLEN cursor (tputc), one tprint at
; the end. A ';' advances the cursor; a bare PRINT is an empty row.
xprint:
        lda #5
        jsr ady
        jsr skipsp
        lda (CPTR),y
        beq xp0
        lda #0
        sta TBLEN
xp1:    jsr pitem
        jsr skipsp
        lda (CPTR),y
        cmp #$3B         ; ';' is the comment char: no char literal for it
        bne xp2
        iny
        jmp xp1
xp2:    lda #0          ; end of list: NUL-terminate and print the row
        ldy TBLEN
        sta TB,y
        lda #<TB
        sta MSGLO
        lda #TB/$100
        sta MSGHI
        jsr tprint
        rts
xp0:    lda #0
        sta TB          ; bare PRINT: empty row
        lda #<TB
        sta MSGLO
        lda #TB/$100
        sta MSGHI
        jsr tprint
        rts

; pitem: one PRINT item, string or numeric. A string item composes
; through strexpr and appends to TB; a numeric one goes through expr
; then nump, whose digits land on the same cursor.
pitem:  jsr skipsp
        lda (CPTR),y
        cmp #'"'
        beq pi_s
        cmp #$41
        bcc pi_n
        cmp #$5B
        bcs pi_n
        iny             ; letter: peek the next char for $
        lda (CPTR),y
        dey
        cmp #'$'
        beq pi_s
pi_n:   jsr expr
        sty T0          ; park the parse index: nump's tputc rewrites y
        jsr nump
        ldy T0
        rts
pi_s:   jsr strexpr
        sty T0          ; park the parse index: the copy runs on x
        ldx #0
pi1:    cpx SLEN
        bcs pi2
        lda SSCR,x
        jsr tputc       ; tputc owns y; x indexes the copy source
        inx
        jmp pi1
pi2:    ldy T0
        rts

; tputc: append A to TB at TBLEN, capped at 31 chars. Clobbers y.
tputc:  ldy TBLEN
        cpy #31
        bcs tpc1        ; full row: truncate silently
        sta TB,y
        inc TBLEN
tpc1:   rts

; nump: ACC -> DBUF (LSB-first via to_dec) -> TB cursor. The final
; tprint is xprint's, so a list shares one row.
nump:   lda ACC
        sta DISPL
        lda ACCH
        sta DISPH
        jsr to_dec
        ldx DLEN
np1:    dex             ; digits come out LSB-first: walk MSB-first
        lda DBUF,x
        jsr tputc       ; tputc owns y, so x indexes DBUF
        cpx #0
        bne np1
        rts

; ady: y += A (tiny shared helper for the keyword skips)
ady:    clc
        sty T0
        clc
        adc T0
        tay
        rts
; --- xlet: LET var = expr or LET var$ = string-expr. y at the L ---------
xlet:   lda #3
        jsr ady         ; past LET
        jsr skipsp
        lda (CPTR),y
        cmp #$41
        bcc xl_e        ; not a letter: ERR
        cmp #$5B
        bcs xl_e
        sec
        sbc #$41
        asl a
        sta VIDX
        iny
        jsr skipsp
        lda (CPTR),y
        cmp #'$'
        beq xlstr
        cmp #'='
        bne xl_e
        iny
        lda VIDX
        pha              ; park the target index: expr's variable terms
        jsr expr         ; overwrite VIDX, so the store can't trust it
        pla
        tax
        lda ACC
        sta VARS,x
        lda ACCH
        sta VARS+1,x
        rts
xlstr:  iny             ; past $
        jsr skipsp
        lda (CPTR),y
        cmp #'='
        bne xl_e
        iny
        jsr strexpr      ; -> SSCR/SLEN
        lda VIDX
        asl a
        asl a            ; VIDX is already *2: *4 = 8-byte slots
        clc
        adc #<STRV
        sta SPTR
        lda #STRV/$100
        adc #0
        sta SPTRH
        ldy #7           ; zero the slot first: shorter values end in NULs
xl0:    lda #0
        sta (SPTR),y
        dey
        bpl xl0
        ldy #0
xl1:    cpy SLEN
        bcs xl2
        lda SSCR,y
        sta (SPTR),y
        iny
        bne xl1
xl2:    rts
xl_e:   jmp xerr
; --- xgoto: GOTO lineno. y at the G. Repositions CPTR to the target -----
xgoto:  lda #4
        jsr ady
        jsr skipsp
        jsr pnum
        jsr findline
        bcs xg1
        jmp xerr
xg1:    lda SRC
        sta CPTR
        lda SRCH
        sta CPTRH
        sec
        rts
; --- xif: IF expr1 op expr2 GOTO lineno. y at the I ---------------------
; Returns C=1 when the condition held and the target line exists.
xif:    lda #2
        jsr ady         ; past IF
        jsr skipsp
        lda (CPTR),y
        cmp #'"'
        bne xi_q        ; not a quote: maybe X$
        jmp xi_s        ; a quote: string condition (xi_s is past a
                        ; short branch's reach, so jmp instead)
xi_q:   cmp #$41
        bcc xi_n        ; below A-Z: numeric
        cmp #$5B
        bcs xi_n        ; past A-Z: numeric
        iny
        lda (CPTR),y
        dey
        cmp #'$'
        bne xi_n        ; plain variable: numeric
        jmp xi_s        ; X$: string condition
xi_n:   jsr expr        ; stops on the comparison char
        lda ACC
        pha             ; LHS parked on the hardware stack: pnum/expr clobber
        lda ACCH        ; every scratch cell, so T0/T0H are not safe
        pha             ; hi on top of lo
        lda (CPTR),y
        cmp #'='
        beq xi1
        cmp #'<'
        beq xi1
        cmp #'>'
        beq xi1
        pla
        pla
        jmp xerr        ; bad op: pop the parked LHS first
xi1:    sta COMP
        iny
        jsr skipsp
        jsr expr        ; RHS -> ACC
        tsx
        lda $0101,x     ; parked LHS hi
        cmp ACCH
        bne xi_dec      ; hi differs: this compare carries the verdict
        lda $0102,x     ; parked LHS lo ($0100,x is the free slot: push
        cmp ACC         ; stores before it decrements, layout is +1/+2)
xi_dec:
        bcc xi_lz
        bne xi_gz
        lda #2          ; equal
        sta T1
        jmp xi_d
xi_lz:  lda #1          ; less
        sta T1
        jmp xi_d
xi_gz:
        lda #3          ; greater
        sta T1
xi_d:   pla
        pla             ; drop the parked LHS before any exit path
xi_sj:  lda COMP        ; the string path joins here: it never parked a
                        ; LHS, so the drop above must not run for it
        cmp #$3D        ; '=' : taken iff verdict == equal
        beq xi_b1
        cmp #$3C        ; '<' : taken iff verdict == less
        beq xi_b2
        lda T1          ; '>' : taken iff verdict == greater
        cmp #3
        beq xi_t
        jmp xi_f
xi_b2:  lda T1
        cmp #1
        beq xi_t
        jmp xi_f
xi_b1:  lda T1
        cmp #2
        beq xi_t
xi_f:   clc
        rts
xi_bad:
        jsr xerr
        sec
        rts
xi_t:   jsr skipsp
        lda (CPTR),y
        cmp #'G'
        bne xi_bad
        lda #4
        jsr ady
        jsr skipsp
        jsr pnum
        jsr findline
        bcs xi_ok
        jsr xerr
        sec
        rts
xi_ok:  sec
        rts

; --- string IF: IF str-expr (=|<|>) str-expr GOTO n ----------------------
; Both sides are full string exprs. The left lands in SSCR and is parked
; in TB (TBLEN), so the right can rebuild SSCR; the compare then walks
; both buffers from 0 — the first differing char decides, and when a
; common prefix runs out the lengths decide. The op dispatch joins the
; numeric path at xi_sj, so the taken/not-taken tail and the direct-IF
; retire are shared.
xi_s:   jsr strexpr      ; LHS -> SSCR/SLEN
        sty T0          ; park the parse index: the copy uses y
        lda #0
        sta TBLEN
        ldy #0
xi_s1:  cpy SLEN
        bcs xi_s2
        lda SSCR,y
        jsr tputc       ; y comes back as TBLEN, in lockstep with the
        iny             ; source index: iny keeps the two in step
        bne xi_s1
xi_s2:  ldy T0
        jsr skipsp
        lda (CPTR),y
        cmp #$3D
        beq xi_s3
        cmp #$3C
        beq xi_s3
        cmp #$3E
        beq xi_s3
        jmp xerr
xi_s3:  sta COMP
        iny
        jsr skipsp
        jsr strexpr      ; RHS -> SSCR/SLEN, y past it (xi_t skips to G)
        ldx #0           ; the compare runs with x so y survives for xi_t:
                         ; xi_t expects y at the char after the condition
                         ; (the 'G' of GOTO), and ldy #0 here used to wipe
                         ; that position — every taken string IF then read
                         ; IBUF[1] or slot text at 0 and hit xi_bad
xi_s4:  cpx TBLEN
        bcs xi_s6        ; LHS exhausted: lengths decide
        cpx SLEN
        bcs xi_s5        ; RHS exhausted: LHS longer -> greater
        lda TB,x
        cmp SSCR,x
        beq xi_s7
        bcc xi_lt        ; LHS char smaller: less
        lda #3
        sta T1
        jmp xi_sj
xi_lt:  lda #1
        sta T1
        jmp xi_sj
xi_s5:  lda #3
        sta T1
        jmp xi_sj
xi_s6:  cpx SLEN
        beq xi_s8        ; both exhausted: equal
        lda #1          ; LHS exhausted, RHS has more: less
        sta T1
        jmp xi_sj
xi_s7:  inx
        jmp xi_s4
xi_s8:  lda #2
        sta T1
        jmp xi_sj
xf_ej:  jmp xf_e         ; branch trampoline: xfor's checks sit past the
                         ; 6502's -128..+127 relative range from xf_e
; --- xfor: FOR var = start TO limit [STEP step]. y at the F -------------
; Pushes (var, limit, step, ret=CPTR+32) onto the 4-level loop stack; the
; start value lands in the variable and the body is the slots after this
; one. NEXT resumes at ret, so the FOR never re-runs.
xfor:   lda #3
        jsr ady          ; past FOR
        jsr skipsp
        lda (CPTR),y
        cmp #$41
        bcc xf_ej
        cmp #$5B
        bcs xf_ej
        sec
        sbc #$41
        asl a
        sta VIDX
        iny
        jsr skipsp
        lda (CPTR),y
        cmp #'='
        bne xf_ej
        iny              ; past '='
        lda VIDX
        pha              ; park the target index: expr's variable terms
        jsr expr         ; clobber VIDX, so the store can't trust it
        pla
        tax
        stx VIDX         ; restore the cell for the record's +0 var store
        lda ACC
        sta VARS,x
        lda ACCH
        sta VARS+1,x     ; var = start
        tya
        pha              ; park y: the record store reuses it as its index
        jsr xf_base      ; SRC = this level's record at FORST + 7*FSP
        ldy #0
        lda VIDX
        sta (SRC),y      ; +0 var
        pla
        tay
        jsr skipsp
        lda (CPTR),y
        cmp #'T'
        bne xf_ej
        lda #2
        jsr ady          ; past TO
        jsr skipsp
        jsr expr         ; limit in ACC
        tya
        pha              ; park y: the record store reuses it as its index
        jsr xf_base      ; re-derive SRC: pnum/expr clobbered it
        ldy #1
        lda ACC
        sta (SRC),y      ; park the limit in the record's own +1/+2: the
        iny              ; STEP parse reuses every scratch cell, and a
        lda ACCH         ; stack park is IRQ-unsafe — a vsync push between
        sta (SRC),y      ; tsx and txs drops the interrupt's saved pc
        pla
        tay
        jsr skipsp
        lda (CPTR),y
        cmp #'S'
        beq xf_s
        lda #1           ; no STEP: step = 1
        sta T0
        lda #0
        sta T1
        jmp xf_rec
xf_s:   lda #4
        jsr ady          ; past STEP
        jsr skipsp
        jsr expr         ; step in ACC
        lda ACC
        sta T0
        lda ACCH
        sta T1
xf_rec: jsr xf_base      ; re-derive SRC
        ldy #3
        lda T0
        sta (SRC),y      ; +3 step lo
        iny
        lda T1
        sta (SRC),y      ; +4 step hi
        iny
        lda CPTR
        clc
        adc #32
        sta (SRC),y      ; +5 ret lo = the slot after the FOR
        iny
        lda CPTRH
        adc #0
        sta (SRC),y      ; +6 ret hi
        inc FSP
        clc              ; fall through into the body
        rts
xf_e:   jmp xerr

; xf_base: SRC/SRCH = FORST + 7*FSP (the record being pushed)
xf_base:
        ldx FSP
        cpx #4
        bcs xf_e         ; loop stack full
        lda mult7,x
        clc
        adc #<FORST
        sta SRC
        lda #FORST/$100
        adc #0
        sta SRCH
        rts
mult7:  .byte 0,7,14,21

; --- xnext: NEXT [var]. Pops the top loop level, steps the variable,
; and repositions CPTR to the FOR's successor (C=1) or falls through
; (C=0) once the limit is passed. y at the N of NEXT ---------------------
xnext:  lda #4
        jsr ady          ; past NEXT
        ldx FSP
        bne xn_hf        ; has a FOR level: pop it
        jmp xn_e         ; NEXT without FOR
xn_hf:  ldx FSP
        dex
        lda mult7,x      ; index the top level but leave FSP: the pop
                         ; happens on exit only, or the next NEXT of a
                         ; continuing loop finds an empty stack
        clc
        adc #<FORST
        sta SRC
        lda #FORST/$100
        adc #0
        sta SRCH         ; SRC/SRCH = the popped level's record
        ldy #0
        lda (SRC),y
        sta T0           ; var index
        ldy #1
        lda (SRC),y
        sta M1
        ldy #2
        lda (SRC),y
        sta M1H          ; M1 = limit
        ldy #3
        lda (SRC),y
        sta T1
        ldy #4
        lda (SRC),y
        sta T1H          ; T1 = step
        ldy #7           ; the record loads left y=4: restore the post-keyword
        jsr skipsp       ; cursor (ady landed here) before skipping spaces
        lda (CPTR),y
        cmp #$41
        bcc xn_add
        cmp #$5B
        bcs xn_add
        sec
        sbc #$41
        asl a
        cmp T0
        bne xn_e
        iny
xn_add: ldx T0
        lda VARS,x
        clc
        adc T1
        sta VARS,x
        lda VARS+1,x
        adc T1H
        sta VARS+1,x     ; var += step
        lda VARS,x
        sta M2
        lda VARS+1,x
        sta M2H          ; M2 = var after the step
        lda T1H
        bmi xn_neg
        sec              ; step >= 0: exit iff limit < var (signed)
        lda M1
        sbc M2
        lda M1H
        sbc M2H
        bvs xn_v1
        bmi xn_exit      ; V=0, N=1: limit < var -> exit
        jmp xn_go        ; V=0, N=0: limit >= var -> continue
xn_v1:  bmi xn_go        ; V=1, N=1: inverted to positive -> continue
        jmp xn_exit      ; V=1, N=0: inverted to negative -> exit
xn_neg: sec              ; step < 0: exit iff var < limit (signed)
        lda M2
        sbc M1
        lda M2H
        sbc M1H
        bvs xn_v2
        bmi xn_exit      ; V=0, N=1: var < limit -> exit
        jmp xn_go
xn_v2:  bmi xn_go        ; V=1, N=1: true positive -> continue
        jmp xn_exit      ; V=1, N=0: true negative -> exit
xn_go:  ldy #5
        lda (SRC),y
        sta CPTR
        ldy #6
        lda (SRC),y
        sta CPTRH        ; resume at the slot after the FOR
        sec
        rts
xn_exit:
        dec FSP          ; NOW pop the level
        clc
        rts              ; fall past NEXT
xn_e:   jmp xerr

; --- xinput: INPUT var|var$. Prints "? " on row 8, reads a line through
; the one-key buffer (backspace works), parses digits or copies a string
xinput: lda #5
        jsr ady          ; past INPUT
        lda #0
        sta SINF         ; numeric unless the target is var$
        jsr skipsp
        lda (CPTR),y
        cmp #$41
        bcc xi_ej
        cmp #$5B
        bcs xi_ej
        sec
        sbc #$41
        asl a
        sta VIDX
        iny
        jsr skipsp
        lda (CPTR),y
        cmp #'$'
        beq xistr_j
        lda #0
        sta IBLEN
        sta IBUF         ; fresh empty line
        jsr rprompt      ; "? " on row 8
xin1:   lda $5800
        beq xin1         ; wait for a key
        cmp #$0D
        beq xin_d
        cmp #$08
        beq xin_b
        cmp #$20
        bcc xin1         ; ignore control codes
        cmp #$7F
        bcs xin1         ; ignore anything past 'Z'
        jsr happend
        jsr rprompt
        jmp xin1
xin_b:  jsr hbksp
        jsr rprompt
        jmp xin1
xi_ej:  jmp xerr         ; branch-range trampolines (dead cell: only
xistr_j: jmp xistr        ; branch targets land here)
xin_d:  lda SINF
        bne xin_s
        lda VIDX
        pha              ; park the target index: pnum clobbers VIDX
        lda CPTR
        pha
        lda CPTRH
        pha              ; park the program cursor: the parse repoints CPTR
        lda #<IBUF
        sta CPTR
        lda #IBUF/$100
        sta CPTRH
        ldy #0
        jsr pnum
        pla
        sta CPTRH
        pla
        sta CPTR         ; restore the cursor: xloop walks slots from CPTR
        pla
        tax
        lda ACC
        sta VARS,x
        lda ACCH
        sta VARS+1,x
        rts
xin_s:  lda VIDX
        asl a
        asl a            ; VIDX is already *2: *4 = 8-byte slots
        clc
        adc #<STRV
        sta SPTR
        lda #STRV/$100
        adc #0
        sta SPTRH
        ldy #7           ; zero the slot: shorter lines end in NULs
xi0:    lda #0
        sta (SPTR),y
        dey
        bpl xi0
        ldy #0
        lda IBLEN
        beq xi3          ; empty Enter: the slot stays ""
xin1c:  cpy #7
        bcs xi3          ; cap at 7 chars
        cpy IBLEN
        bcs xi3
        lda IBUF,y
        sta (SPTR),y
        iny
        bne xin1c
xi3:    rts
xistr:  lda #1
        sta SINF
        lda #0
        sta IBLEN
        sta IBUF         ; fresh empty line
        jsr rprompt
        jmp xin1         ; same prompt/key loop as the numeric path
xi_e:   jmp xerr

; --- strings: A$-Z$ (7 chars max), quoted literals, + concat, LEN() -----
; strexpr: compose a string expression into SSCR with length SLEN.
; Grammar: sfactor { + sfactor }. y is the parse index into (CPTR),y on
; entry and on exit (past the last factor). The numeric side calls this
; only from flen, where the running sum is parked on the hardware stack.
strexpr:
        lda #0
        sta SLEN
        sta SSCR        ; SSCR[0] = NUL: an empty compose is ""
sx0:    jsr sfactor
        jsr skipsp
        lda (CPTR),y
        cmp #'+'
        bne sx1
        iny
        jmp sx0
sx1:    rts

; sfactor: a quoted literal or a string variable. Anything else is left
; unconsumed (y untouched) so the numeric parser can take over.
sfactor:
        jsr skipsp
        lda (CPTR),y
        cmp #'"'
        bne sf1
        iny             ; literal: copy chars until the closing quote
sf0:    lda (CPTR),y
        beq sf0u        ; NUL before the quote: unterminated
        cmp #'"'
        beq sf0e
        sty T0          ; park the parse index: sputc rewrites y
        jsr sputc
        ldy T0
        iny
        jmp sf0
sf0u:   jmp fpar_e
sf0e:   iny
        rts
sf1:    cmp #$41
        bcc sf9         ; not a letter: leave it unconsumed
        cmp #$5B
        bcs sf9
        iny             ; peek the next char for $
        lda (CPTR),y
        dey
        cmp #'$'
        bne sf9         ; a plain letter: the numeric factor owns it
        ; string variable: slot at STRV + 8*(c-'A'). VIDX belongs to the
        ; numeric LET/INPUT side, so the slot pointer is built locally
        sty T0          ; park the parse index through the copy
        lda (CPTR),y    ; reload the letter: the $ peek clobbered A
        sec
        sbc #$41
        asl a
        asl a
        asl a           ; *8 = 8-byte slots
        clc
        adc #<STRV
        sta SPTR
        lda #STRV/$100
        adc #0
        sta SPTRH
        lda T0
        clc
        adc #2          ; past the letter and $: T0 = the parse index
        sta T0          ; past "X$", so sf3 restores the consumed index
        ldy #0          ; (zp),y reads the slot; sputc owns y, so the
sf2:    lda (SPTR),y    ; source index reloads from SLEN each append
        beq sf3         ; slot NUL: copied it all
        jsr sputc
        ldy SLEN        ; SLEN froze at 7 if truncating: loop still ends
        cpy #7
        bcc sf2
sf3:    ldy T0
        rts
sf9:    rts             ; not a string factor

; sputc: append A to SSCR at SLEN, capped at 7 chars. Clobbers y.
sputc:  ldy SLEN
        cpy #7
        bcs sp0         ; full: truncate silently
        sta SSCR,y
        iny
        sty SLEN
sp0:    rts

; --- xpoke: POKE addr, val. y at the P of POKE ---------------------------
xpoke:  lda #4
        jsr ady          ; past POKE
        jsr skipsp
        jsr expr         ; address
        lda ACC
        pha
        lda ACCH
        pha              ; park the address on the hardware stack
        jsr skipsp
        lda (CPTR),y
        cmp #','
        bne pk_e
        iny
        jsr skipsp
        jsr expr         ; value
        lda ACC
        sta T1
        lda ACCH
        sta T1H
        pla
        sta T0H
        pla
        sta T0           ; pull order: ACCH parked last, so hi comes off first
        ldy #0
        lda T1
        sta (T0),y       ; full-bus write: RAM, framebuffer, I/O all take it
        rts
pk_e:   jmp xerr

; --- findline: target line in ACC -> C=1 found, CPTR = slot base --------
findline:
        ldx #0
fl1:    cpx NUMPROG
        bcs fl_nf        ; scan ended: not found
        txa
        jsr slotptr
        ldy #0
        lda (SRC),y
        cmp ACC
        bne fl2
        iny
        lda (SRC),y
        cmp ACCH
        bne fl2
        lda SRC
        sta CPTR
        lda SRCH
        sta CPTRH
        sec
        rts
fl2:    inx
        jmp fl1
fl_nf:  clc
        rts

; --- slotptr: A = slot index 0..31 -> SRC/SRCH = PROG + 32*A -------------
slotptr:
        sta T1
        lda #0
        sta T1H
        asl T1
        rol T1H          ; x2
        asl T1
        rol T1H          ; x4
        asl T1
        rol T1H          ; x8
        asl T1
        rol T1H          ; x16
        asl T1
        rol T1H          ; x32
        lda #<PROG
        clc
        adc T1
        sta SRC
        lda #PROG/$100
        adc T1H
        sta SRCH
        rts
; --- xstore: LN/LNH + text at IBUF,TXI -> sorted insert or overwrite ----
xstore:
        sty STXV         ; hsubmit hands us the text-start index in Y
        ldx #0
xs_scan:
        cpx NUMPROG
        bcs xs_ins       ; past the last line: append (index = X)
        txa
        jsr slotptr
        ldy #1
        lda (SRC),y
        cmp LNH
        bne xs_dec
        dey
        lda (SRC),y
        cmp LN
xs_dec:
        bcc xs_nx        ; slot line < new line: keep scanning
        beq xs_ovw       ; equal: overwrite in place
        jmp xs_ins       ; slot line > new line: insert here
xs_nx:  inx
        jmp xs_scan
xs_ovw:
        stx LEN
        jmp xs_fill
xs_ins:
        stx LEN
        lda NUMPROG
        cmp #32
        bcs xs_err
        ldx NUMPROG
        beq xs_up        ; empty store: nothing to shift, fill at 0
xs_sh:  dex
        cpx LEN
        beq xs_up
        bcc xs_up
        txa
        jsr slotptr      ; SRC = slot X
        lda SRC
        clc
        adc #32
        sta DST
        lda SRCH
        adc #0
        sta DSTH         ; DST = SRC + 32
        ldy #31
xs_sh2:
        lda (SRC),y
        sta (DST),y
        dey
        bpl xs_sh2
        jmp xs_sh
xs_up:  inc NUMPROG
        lda PROGTOP
        clc
        adc #32
        sta PROGTOP
        bcc xs_fill
        inc PROGEH
xs_fill:
        lda LEN
        jsr slotptr
        ldy #0
        lda LN
        sta (SRC),y
        iny
        lda LNH
        sta (SRC),y
        ldx STXV
        ldy #3
xs_w1:  cpy #32
        bcs xs_wd
        lda IBUF,x
        beq xs_wd
        sta (SRC),y
        iny
        inx
        jmp xs_w1
xs_wd:  lda #0
        sta (SRC),y      ; NUL pins the text length
        tya
        sec
        sbc #3
        ldy #2
        sta (SRC),y      ; LEN
        rts
xs_err:
        jmp xerr

; --- dline: delete line LN/LNH (bare line number typed) -----------------
dline:  ldx #0
dl_s:   cpx NUMPROG
        bcs dl_end
        txa
        jsr slotptr
        ldy #1
        lda (SRC),y
        cmp LNH
        bne dl_nx
        dey
        lda (SRC),y
        cmp LN
        bne dl_nx
        jmp dl_sh
dl_nx:  inx
        jmp dl_s
dl_sh:  inx              ; X = source slot k, copy down to k-1
dl_sh1: cpx NUMPROG
        bcs dl_dec
        txa
        jsr slotptr      ; SRC = slot X
        lda SRC
        sec
        sbc #32
        sta DST
        lda SRCH
        sbc #0
        sta DSTH         ; DST = SRC - 32
        ldy #0
dl_c1:  lda (SRC),y
        sta (DST),y
        iny
        cpy #32
        bcc dl_c1
        inx
        jmp dl_sh1
dl_dec: dec NUMPROG
        lda PROGTOP
        sec
        sbc #32
        sta PROGTOP
        bcs dl_end
        dec PROGEH
dl_end: rts
; --- xloop: run statements from CPTR until PROGTOP, END or ERR ----------
xloop:  lda ERRF
        bne xdone
        lda PROGEH
        cmp CPTRH
        bcc xdone
        bne xl_r1
        lda PROGTOP
        cmp CPTR
        bcc xdone
        beq xdone
xl_r1:  jsr xstmt
        bcs xloop        ; repositioned: re-run the checks at the top
        lda CPTR
        clc
        adc #32
        sta CPTR
        bcc xloop
        inc CPTRH
        jmp xloop
xdone:  rts

; --- direct R (RUN), N (NEW) -------------------------------------------
drun:   lda NUMPROG
        beq drun0        ; nothing to run: clear the line, back to direct mode
        lda #0
        sta CPTR
        lda #PROG/$100
        sta CPTRH
        lda #0
        sta FSP          ; a run starts with no live loops and no return
        sta GSP          ; frames (xloop is re-entered per statement, so this
        jsr rdinit       ; frames; data cursor rewinds per run, and per run-start
        jsr xloop        ; in the direct GOTO/IF run paths too
        jmp hcln
drun0:  jsr hcln
        rts

xnew:   lda #0
        sta NUMPROG
        sta PROGTOP
        lda #PROG/$100
        sta PROGEH
        ldx #0
xn1:    lda #0
        sta VARS,x
        inx
        cpx #52
        bcc xn1
        jsr szero
        jsr hcln
        rts

; szero: clear the 26 string slots (208 bytes, exactly one page).
szero:  lda #0
        tax
sz_l:   sta $1200,x
        inx
        bne sz_l
        rts

; --- xerr: print ERR and abort the current run --------------------------
xerr:   lda #<errmsg
        sta MSGLO
        lda #errmsg/$100
        sta MSGHI
        jsr tprint
        lda #0
        sta FSP          ; a failed run leaves no live loops behind
        sta GSP          ; ...nor return frames
        lda #1
        sta ERRF
        rts

; --- draw_msg / clear_scr: calc.s ports -------------------------------
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


; --- mul16 / div16 / to_dec: calc.s ports, cells match basic's zpage --
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
; --- static strings ----------------------------------------------------
readymsg:
        .text "VINTAGE-1 BASIC"
        .byte 0
readymgs2:
        .text "READY"
        .byte 0
errmsg:
        .text "ERR"
        .byte 0

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

; --- xend: END terminates the run by parking CPTR at PROGTOP ------------
xend:   lda PROGTOP
        sta CPTR
        lda PROGEH
        sta CPTRH
        sec
        rts

; --- xlist: LIST -> every slot as "NNNN TEXT" rows ----------------------
xlist:  lda #0
        sta TIDX
xl_l:   ldx TIDX
        cpx NUMPROG
        bcs xl_9
        txa
        jsr slotptr
        ldy #0
        lda (SRC),y
        sta DISPL
        iny
        lda (SRC),y
        sta DISPH
        jsr to_dec
        ldy DLEN
        ldx #0
xl_d:   dey
        lda DBUF,y
        sta TB,x
        inx
        cpy #0
        bne xl_d
        lda #' '
        sta TB,x
        inx
        ldy #3
xl_t:   lda (SRC),y
        beq xl_w
        sta TB,x
        iny
        inx
        jmp xl_t
xl_w:   lda #0
        sta TB,x
        lda #<TB
        sta MSGLO
        lda #TB/$100
        sta MSGHI
        jsr tprint
        inc TIDX
        jmp xl_l

xl_9:   rts

; --- xgosub / xret: GOSUB lineno / RETURN --------------------------------
; Lives past the font ($E000 region is full). A frame is [ret lo @+0,
; ret hi @+4, FSP @+8], 4 levels deep. The FSP snapshot makes RETURN
; discard FOR levels opened inside the callee while the caller's stay
; live. The return address is parked on the CPU stack across findline,
; which re-points CPTR at the found slot — reading CPTR after the call
; would push the callee's slot instead of the GOSUB's successor.
xgosub:
        lda #5
        jsr ady          ; past GOSUB
        jsr skipsp
        jsr pnum
        lda CPTR         ; park CPTR+32 (the return slot) over findline
        clc
        adc #32
        pha
        lda CPTRH
        adc #0
        pha
        jsr findline
        bcs gs1
        pla              ; no such line: unwind the park, ERR
        pla
        jmp xerr
gs1:    pla
        sta T0H          ; the parked return address
        pla
        sta T0
        ldx GSP
        cpx #4
        bcs gs_of        ; deeper than 4: ERR
        lda T0
        sta GSTK,x
        lda T0H
        sta GSTK+4,x
        lda FSP
        sta GSTK+8,x     ; snapshot: RETURN discards the callee's FOR levels
        inc GSP
        sec              ; CPTR already points at the callee (findline set it)
        rts
gs_of:  jsr xerr
        sec
        rts
xret:   ldx GSP
        beq gs_e         ; RETURN without GOSUB: ERR
        dex
        stx GSP
        lda GSTK,x
        sta CPTR
        lda GSTK+4,x
        sta CPTRH
        lda GSTK+8,x
        sta FSP          ; restore the caller's FOR depth
        sec
        rts
gs_e:   jsr xerr
        sec
        rts

; --- DATA/READ/RESTORE ----------------------------------------------------
; The data cursor RDLO/RDHI walks the program store slot by slot: a slot
; base address means "scanning for the next DATA line", anything else
; means "mid-list inside one". Every run start (drun, direct GOTO/IF) and
; RESTORE rewind it to PROG. READ takes literals only — no expressions —
; and type-mismatched items ERR just like malformed ones.
;
; drun / direct-run paths: rdinit rewinds the cursor.

rdinit: lda #0
        sta RDLO
        lda #PROG/$100
        sta RDHI
        rts

; xd3: DATA reached in execution order — the data itself is inert; skip it.
; Reached by jmp from xstmt with y at the keyword's first letter.
xd3:    iny
        lda (CPTR),y     ; 2nd char of DATA
        dey
        cmp #'A'
        bne xd3e
        clc
        rts
xd3e:   jmp xerr

; xr3: the R-family 3rd-char dispatch, reached by jmp from xstmt with y at
; the keyword. Handlers run and return to xloop via the stacked return.
xr3:    iny
        iny
        lda (CPTR),y     ; 3rd char: A = READ, S = RESTORE, T = RETURN
        dey
        dey
        cmp #'T'
        bne xr3a
        jsr xret
        sec
        rts
xr3a:   cmp #'A'
        bne xr3b
        jsr xread
        clc
        rts
xr3b:   cmp #'S'
        bne xr3e
        jsr xrestore
        clc
        rts
xr3e:   jmp xerr

; xrestore: rewind the data cursor to the first slot.
xrestore:
        lda #0
        sta RDLO
        lda #PROG/$100
        sta RDHI
        rts

xr1e:   jmp xerr        ; branch trampoline (dead cell: only branch targets land here)

; xread: READ var[,var...]. Per item: parse the variable (VIDX = its slot),
; park CPTR/y on the CPU stack — rdnext/rdnexts repoint CPTR at the DATA
; text — pull the next literal, restore, and store into the variable's
; slot. Numeric items go through pnum (optional '-'); string items copy
; into the 8-byte STRV slot via SPTR exactly like LET's string store.
xread:  lda #4
        jsr ady          ; past READ
        jsr skipsp
xr1:    lda (CPTR),y
        cmp #$41
        bcc xr1e
        cmp #$5B
        bcs xr1e
        sec
        sbc #$41
        asl a
        sta VIDX         ; rdnext never touches VIDX, so it survives the call
        iny
        jsr skipsp
        lda (CPTR),y
        cmp #'$'
        beq xr_str
        ; numeric: park CPTR/y, fetch the literal, restore, store
        lda CPTR
        pha
        lda CPTRH
        pha
        tya
        pha
        jsr rdnext
        pla
        tay
        pla
        sta CPTRH
        pla
        sta CPTR
        lda VIDX
        tax
        lda ACC
        sta VARS,x
        lda ACCH
        sta VARS+1,x
        jmp xr_next
xr_str: iny              ; past $
        lda CPTR
        pha
        lda CPTRH
        pha
        tya
        pha              ; y parked: the STRV store uses y freely
        jsr rdnexts      ; -> SSCR/SLEN
        ; store into the STRV slot: zero it first, then SLEN bytes
        lda VIDX
        asl a
        asl a            ; VIDX is already *2: *4 = 8-byte slots
        clc
        adc #<STRV
        sta SPTR
        lda #STRV/$100
        adc #0
        sta SPTRH
        ldy #7           ; zero the slot first: shorter values end in NULs
xr0:    lda #0
        sta (SPTR),y
        dey
        bpl xr0
        ldy #0
xr1s:   cpy SLEN
        bcs xr2s
        lda SSCR,y
        sta (SPTR),y
        iny
        bne xr1s
xr2s:   pla
        tay
        pla
        sta CPTRH
        pla
        sta CPTR
        jmp xr_next
xr_e:   jmp xerr

; xr_next: per-variable advance: ',' loops to the next variable, NUL ends.
xr_next:
        jsr skipsp
        lda (CPTR),y
        cmp #','
        bne xr_next1
        iny
        jsr skipsp
        jmp xr1
xr_next1:
        cmp #0
        bne xr_ne1
        rts
xr_ne1: jmp xerr

; rdnext: pull the next DATA item as a number into ACC (optional '-'),
; then advance the data cursor via rdadv. A quoted or bare-word item —
; anything that isn't a digit run — ERRs: type mismatches ERR, they
; don't coerce.
rdnext: jsr rditem
        lda (CPTR),y
        cmp #'"'
        beq rdn_bad
        cmp #'-'
        bne rdn_num
        iny
        jsr pnum
        sec              ; negate: ACC = 0 - ACC
        lda #0
        sbc ACC
        sta T0
        lda #0
        sbc ACCH
        sta ACCH
        lda T0
        sta ACC
        jmp rdadv
rdn_num:
        jsr pnum
        jmp rdadv
rdn_bad:
        jmp xerr

; rdnexts: pull the next DATA item as a string into SSCR/SLEN. A leading
; quote reads to the closing quote (chars past 7 are skipped but the item
; still must be terminated); an unquoted item reads raw to ',' or NUL with
; trailing spaces trimmed. Then advance the data cursor via rdadv.
rdnexts:
        jsr rditem
        lda (CPTR),y
        cmp #'"'
        beq rds_q
        ; unquoted: raw chars to ',' or NUL, capped at 7 stored
        ldx #0
rds1:   lda (CPTR),y
        cmp #','
        beq rds_t
        cmp #0
        beq rds_t
        cpx #7
        bcc rds1s
        iny              ; over cap: skip the char, keep scanning for the end
        jmp rds1
rds1s:  sta SSCR,x
        inx
        iny
        jmp rds1
rds_t:  ; trailing-space trim: walk x back while the last char is a space
        cpx #0
        beq rds_z
        lda SSCR-1,x     ; SSCR-1+x = SSCR[x-1]: the last stored char
        cmp #' '
        bne rds_z
        dex
        jmp rds_t
rds_z:  stx SLEN
        jmp rdadv
rds_q:  iny              ; past the opening quote
        ldx #0
rds2:   lda (CPTR),y
        cmp #'"'
        beq rds2e
        cmp #0
        beq rds_e2       ; unterminated: ERR
        cpx #7
        bcc rds2s
        iny              ; over cap: skip but keep looking for the quote
        jmp rds2
rds2s:  sta SSCR,x
        inx
        iny
        jmp rds2
rds2e:  stx SLEN
        iny              ; past the closing quote
        jmp rdadv
rds_e2: jmp xerr

; rditem: position the cursor at the next DATA item: CPTR/CPTRH = the
; position, y = 0. Scan mode (RDLO & $1F == 0) checks the end of the
; program first (PROGTOP/PROGEH), then per slot looks for text starting
; with "DATA" (text at +3), steps RDPTR by 32 and loops. Found: RDLO += 7
; (past "DATA") and parse from there. Mid-list entry: CPTR = RDPTR, skipsp.
rditem: lda RDLO
        and #$1F
        bne rdi_mid
        ; scan mode: end check against the one-past-last-line pointer
        lda PROGEH
        cmp RDHI
        bcc rdi_out
        bne rdi_go
        lda PROGTOP
        cmp RDLO
        bcc rdi_out
        beq rdi_out
        ; slot keyword check: SRC = RDPTR, bytes +3..+6 must be D,A,T,A
rdi_go: lda RDLO
        sta SRC
        lda RDHI
        sta SRCH
        ldy #3
        ldx #0
rdi_s1: lda (SRC),y
        cmp DTXT,x
        bne rdi_n
        inx
        iny
        cpx #4
        bcc rdi_s1
        ; matched "DATA": cursor = slot+7 (32-aligned base + 7 can't carry)
        lda #7
        clc
        adc RDLO
        sta RDLO
        jmp rdi_mid
rdi_n:  ; not DATA: next slot. RDLO+32 may carry into RDHI.
        lda RDLO
        clc
        adc #32
        sta RDLO
        bcc rdi_scan
        inc RDHI
rdi_scan:
        jmp rditem
rdi_out:
        jmp xerr
rdi_mid:
        lda RDLO
        sta CPTR
        lda RDHI
        sta CPTRH
        ldy #0
        jmp skipsp

; rdadv: after an item, the data cursor must end on a slot base (list
; finished) or just past a ','. skipsp first, then: ',' -> RDPTR = CPTR+y;
; NUL -> slot base + 32. Anything else is malformed data: ERR.
rdadv:  jsr skipsp
        lda (CPTR),y
        cmp #','
        bne rdadv1
        iny
        ; RDPTR = CPTR + y (16-bit)
        lda CPTR
        sta RDLO
        lda CPTRH
        sta RDHI
        tya
        clc
        adc RDLO
        sta RDLO
        lda #0
        adc RDHI
        sta RDHI
        rts
rdadv1: cmp #0
        bne rdadv_e
        ; NUL: next slot base = (CPTR & $E0) + 32, carry into the hi byte
        lda CPTR
        and #$E0
        clc
        adc #32
        sta RDLO
        lda #0
        adc CPTRH
        sta RDHI
        rts
rdadv_e:
        jmp xerr

DTXT:   .text "DATA"

; --- PLOT / UNPLOT: set or clear one pixel at expr x, expr y -----------
; x must be 0..255, y 0..191 — anything else ERRs. The bit lands at
; SCREEN + y*32 + x/8, mask $80 >> (x&7) (MSB leftmost). PLOT and UNPLOT
; share the core; XPLOTF picks ora vs and at the modify. Both parse from
; y at the keyword's first letter (both keywords are 4 chars).
xplotf: lda #1
        .byte $2C       ; BIT abs eats 2 bytes: PLOT falls through to the
xunplf: lda #0          ; store with A=1; UNPLOT enters at the lda with A=0
        sta XPLOTF
        jsr plxy        ; parse "x,y" -> PX, PLY
        ; bit mask: $80 >> (PX & 7) — the count-0 case can't share the
        ; shift loop (X=0 would run it 256 times), so it gets its own load
        lda PX
        and #7
        beq xpmk0        ; x&7 = 0: mask stays $80
        tax
        lda #$80
xpmk:   lsr a
        dex
        bne xpmk
        jmp xpmk1
xpmk0:  lda #$80
xpmk1:  sta T1          ; mask parked while the address is built
        ; address: SRC/SRCH = SCREEN + PLY*32 + PX/8
        lda PLY
        sta T0
        lda #0
        sta T0H
        asl T0
        rol T0H         ; y*2
        asl T0
        rol T0H         ; y*4
        asl T0
        rol T0H         ; y*8
        asl T0
        rol T0H         ; y*16
        asl T0
        rol T0H         ; y*32
        lda PX
        lsr a
        lsr a
        lsr a           ; x/8
        clc
        adc T0
        sta SRC
        lda #0
        adc T0H         ; carry from the lo-byte add
        clc
        adc #SCREEN/$100
        sta SRCH
        ; read-modify-write the one bit
        ldy #0
        lda (SRC),y
        ldx XPLOTF
        bne xpset
        eor #$FF
        and T1
        jmp xpsto
xpset:  ora T1
xpsto:  sta (SRC),y
        clc
        rts

; plxy: parse "x,y" — two full expressions, ','-separated — into PX/PLY.
; Enters with y at the keyword's first letter; both keywords are 4 chars.
; x must land 0..255 and y 0..191: a high byte or y >= 192 is ERR.
plxy:   lda #4
        jsr ady         ; past the keyword
        jsr skipsp
        jsr expr
        lda ACCH
        bne plerr
        lda ACC
        sta PX
        jsr skipsp
        lda (CPTR),y
        cmp #','
        bne plerr
        iny
        jsr skipsp
        jsr expr
        lda ACCH
        bne plerr
        lda ACC
        cmp #192
        bcs plerr
        sta PLY
        rts
plerr:  jmp xerr

; --- direct-mode PLOT/UNPLOT: CPTR=IBUF, parse from the line start -----
; Same shape as dpk_j/din_j/dprint up in E000, but living here where
; there's room; E000 keeps only 3-byte trampolines.
dplf:   lda #<IBUF
        sta CPTR
        lda #IBUF/$100
        sta CPTRH
        ldy #0
        jsr xplotf       ; far PLOT entry: parse "x,y", set the bit
        jmp hcln
dupf:   lda #<IBUF
        sta CPTR
        lda #IBUF/$100
        sta CPTRH
        ldy #0
        jsr xunplf       ; far UNPLOT entry: clear it again
        jmp hcln
