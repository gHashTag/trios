# PhD v5.2 Prose Stylebook (Helen Sword + Pinker discipline)

## Mission
Make every chapter open with a hook, breathe with concrete imagery, and end with a clear takeaway — without removing a single existing theorem, table, citation, or constant.

## What to ADD per chapter (always additive, never delete content)

Right AFTER `\begin{figure} ... \end{figure}` and BEFORE `\section{Abstract}\label{abstract}`, insert:

1. **One epigraph block** — a real quotation, attributed, italic, in `\begin{quote}\itshape ... \end{quote}`. Pick a quote that genuinely fits the chapter topic. Authors to draw from: Hardy, Knuth, Iverson, Dijkstra, Pinker, Sword, Popper, Feynman, Shannon, Turing, Lamport, Tarjan, Hoare, Wittgenstein, Bachelard, Lakatos, Galison, Karen Uhlenbeck, Lee Smolin, Coldea, Penrose, Conway, Kepler, Lucas, Vogel, Fibonacci historian Rachel Levy, Deming, Goodhart, Kahneman, McIlroy, Stallman, Linus Torvalds, internal Trinity Ops, Trinity Runtime Notes.

2. **One `\section*{...}` "narrative opener"** with a 2-4 paragraph prose introduction. Style rules:
   - First sentence: a hook (story, surprise, concrete number, a vivid contrast).
   - Use ordinary words for complex things. Avoid jargon density.
   - Give a road-sign sentence near the end: "the rest of this chapter is about ...".
   - Use em-dashes generously; prefer short sentences after long ones (rhythm).
   - Never use "this paper" — say "this chapter" or "this book".
   - Always preserve `φ² + φ⁻² = 3` framing where it naturally fits.
   - 200-450 words.

## What NOT to touch
- Do NOT modify `\section{Abstract}\label{abstract}` and downstream sections.
- Do NOT modify `\chapter{...}`, `\label{...}`, `\addcontentsline{...}`, `\begin{figure}...\end{figure}`.
- Do NOT touch any `\filepath{...}`, `\verb|...|`, `\begin{verbatim}...\end{verbatim}`, `\begin{tabularx}`, `\begin{longtable}`, math `\[...\]`, `\begin{align*}`, citations `[1]`, `\cite{...}`, theorems, or numerical constants.
- Do NOT delete or shorten existing prose. Insertion only.

## LaTeX rules to obey
- No bare `_` or `&` outside math/verb (escape as `\_`, `\&`).
- No `$...$` or `$$...$$` — use `\(...\)` and `\[...\]`.
- Quotes: `` ``...'' `` (left/right pairs).
- Em-dash: `---`. En-dash: `--`.
- Apostrophe: `'`.
- Russian-language chapters do NOT exist as separate files; the same `ch_NN.tex` is included by both EN and RU builds, so write narrative openers in **English** (the way ch_01 / ch_03 / ch_08 etc. already are).

## Build constraints
- Total page budget MUST stay 600-700p — do not balloon any single chapter beyond +1 page (~450 words narrative cap).
- Max overfull \hbox must remain < 100pt.

## Coverage map
- Tier-A heavy chapters (already done in v5.1): ch_01, ch_03, ch_08, ch_15, ch_21, ch_24, ch_27 — DO NOT MODIFY.
- Tier-B chapters to improve in v5.2 (28 total): ch_00, ch_02, ch_04, ch_05, ch_06, ch_07, ch_09, ch_10, ch_11, ch_12, ch_13, ch_14, ch_16, ch_17, ch_18, ch_19, ch_20, ch_22, ch_23, ch_25, ch_26, ch_28, ch_29, ch_30, ch_31, ch_32, ch_33, ch_34.
- Tier-C appendices (12): A-catalogue, B-falsification, C-golden-benchmark, D-golden-mirror, E-lexicon, F-coq-citation-map, F-fpga-bitstream, G-data-availability, H-acm-ae-checklist, H-zenodo-doi, I-xdc-pin-map, J-troubleshooting, K-agent-memory, L-pollen-channel.
  - For appendices, opener can be smaller (1-2 paragraphs, 100-250 words). Insert AFTER any `\appendix` or chapter-like header but BEFORE the first `\section` or main content.

## Anchor reminders to weave naturally (don't force into every chapter)
- φ² + φ⁻² = 3
- Lucas pair L₇=29, L₈=47
- BPB Gate-2 ≤ 1.85, Gate-3 ≤ 1.5
- 47M-param model, FPGA QMTech XC7A100T at 92 MHz, < 1W
- 297 closed Coq Qed theorems / 65 .v files
- DARPA 3000× energy ratio
- Fibonacci seeds F17=1597, F18=2584, F19=4181, F20=6765, F21=10946

## Title titles for narrative `\section*{...}` (suggestions, agents may invent better)
Make them concrete and image-rich. Avoid abstract noun-phrase titles like "Background and Motivation".
Good: "Two clocks, no resonance", "The number that everything is about", "Three values, one machine".
Bad: "Introduction", "Theoretical context", "On methodology".

## Submission contract
After editing each file, the agent MUST verify the file still contains its original `\section{Abstract}\label{abstract}` line and original `\chapter{...}` line unchanged. If any check fails, revert.
