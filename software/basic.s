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
TB     = $2740         ; 33-byte compose buffer for LIST lines

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

; --- boot: clear everything, print READY, then the poll loop ------------
start:  jsr clear_scr
        jsr tclear
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
xlj_j:  jmp xlist
dexec:  lda IBUF
        cmp #'R'
        beq drr_j
        cmp #'N'
        beq dnew
        cmp #'E'         ; END direct: no-op
        beq hdone
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
        beq dgoto_j
        cmp #'I'
        beq difi
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
        jmp xloop
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
        jmp xprint
xs1:    cmp #'L'
        bne xs2
        jsr xlet
        clc
        rts
xs2:    cmp #'G'
        bne xs3
        jsr xgoto
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
xs7:    jmp xerr       ; unknown keyword: ERR, abort
xpk_j:  jsr xpoke
        clc
        rts
xin_j:  jsr xinput
        clc
        rts
; --- xprint: PRINT expr | PRINT. y at the P of the keyword --------------
; A bare PRINT prints an empty row. The value goes through to_dec into
; DBUF (LSB-first), is reversed MSB-first into TB, and tprint-ed.
xprint:
        lda #5
        jsr ady
        jsr skipsp
        lda (CPTR),y
        beq xp0
        jsr expr
        jsr nump
        rts
xp0:    lda #0
        sta TB          ; bare PRINT: empty row
        lda #<TB
        sta MSGLO
        lda #TB/$100
        sta MSGHI
        jsr tprint
        rts

; nump: DISPL/DISPH = ACC -> DBUF -> TB -> tprint
nump:   lda ACC
        sta DISPL
        lda ACCH
        sta DISPH
        jsr to_dec
        ldy DLEN
        ldx #0
npl:    dey
        lda DBUF,y
        sta TB,x
        inx
        cpy #0
        bne npl
        lda #0
        sta TB,x
        lda #<TB
        sta MSGLO
        lda #TB/$100
        sta MSGHI
        jsr tprint
        rts

; ady: y += A (tiny shared helper for the keyword skips)
ady:    clc
        sty T0
        clc
        adc T0
        tay
        rts
; --- xlet: LET var = expr. y at the L of the keyword --------------------
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
        jsr expr        ; stops on the comparison char
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
        lda COMP
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

; --- xinput: INPUT var. Prints "? " on row 8, reads a line through the
; one-key buffer (backspace works), parses digits into the variable -----
xinput: lda #5
        jsr ady          ; past INPUT
        jsr skipsp
        lda (CPTR),y
        cmp #$41
        bcc xi_e
        cmp #$5B
        bcs xi_e
        sec
        sbc #$41
        asl a
        sta VIDX
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
xin_d:  lda VIDX
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
xi_e:   jmp xerr

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
        beq dl_end       ; nothing to run: back to direct mode
        lda #0
        sta CPTR
        lda #PROG/$100
        sta CPTRH
        lda #0
        sta FSP          ; a run starts with no live loops (xloop is re-entered
        jsr xloop        ; per statement, so this can't live at its top)
        jmp hcln

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
        jsr hcln
        rts

; --- xerr: print ERR and abort the current run --------------------------
xerr:   lda #<errmsg
        sta MSGLO
        lda #errmsg/$100
        sta MSGHI
        jsr tprint
        lda #0
        sta FSP          ; a failed run leaves no live loops behind
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
