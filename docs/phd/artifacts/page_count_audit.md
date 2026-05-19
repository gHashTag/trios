# PhD Page Count Audit — as of 2026-05-14

## Method

```
git ls-tree -r origin/main --name-only -- docs/phd/chapters/ docs/phd/appendix/
# then for each file:
git show origin/main:<path> | wc -l
```

Estimated pages = total LoC / 30 (rough conversion).

---

## Chapter Files

| File | LoC |
|------|-----|
| docs/phd/chapters/flos_00.tex | 169 |
| docs/phd/chapters/flos_01.tex | 1527 |
| docs/phd/chapters/flos_02.tex | 331 |
| docs/phd/chapters/flos_03.tex | 445 |
| docs/phd/chapters/flos_04.tex | 401 |
| docs/phd/chapters/flos_05.tex | 1522 |
| docs/phd/chapters/flos_06.tex | 501 |
| docs/phd/chapters/flos_07.tex | 391 |
| docs/phd/chapters/flos_08.tex | 456 |
| docs/phd/chapters/flos_09.tex | 427 |
| docs/phd/chapters/flos_10.tex | 431 |
| docs/phd/chapters/flos_11.tex | 356 |
| docs/phd/chapters/flos_12.tex | 394 |
| docs/phd/chapters/flos_13.tex | 1644 |
| docs/phd/chapters/flos_14.tex | 303 |
| docs/phd/chapters/flos_15.tex | 398 |
| docs/phd/chapters/flos_16.tex | 350 |
| docs/phd/chapters/flos_17.tex | 366 |
| docs/phd/chapters/flos_18.tex | 427 |
| docs/phd/chapters/flos_19.tex | 321 |
| docs/phd/chapters/flos_20.tex | 520 |
| docs/phd/chapters/flos_21.tex | 443 |
| docs/phd/chapters/flos_22.tex | 422 |
| docs/phd/chapters/flos_23.tex | 341 |
| docs/phd/chapters/flos_24.tex | 439 |
| docs/phd/chapters/flos_25.tex | 408 |
| docs/phd/chapters/flos_26.tex | 446 |
| docs/phd/chapters/flos_27.tex | 436 |
| docs/phd/chapters/flos_28.tex | 411 |
| docs/phd/chapters/flos_29.tex | 451 |
| docs/phd/chapters/flos_30.tex | 395 |
| docs/phd/chapters/flos_31.tex | 356 |
| docs/phd/chapters/flos_32.tex | 324 |
| docs/phd/chapters/flos_33.tex | 339 |
| docs/phd/chapters/flos_34.tex | 184 |
| docs/phd/chapters/flos_35.tex | 567 |
| docs/phd/chapters/flos_36.tex | 446 |
| docs/phd/chapters/flos_37.tex | 512 |
| docs/phd/chapters/flos_38.tex | 395 |
| docs/phd/chapters/flos_39.tex | 483 |
| docs/phd/chapters/flos_40.tex | 260 |
| docs/phd/chapters/flos_41.tex | 197 |
| docs/phd/chapters/flos_42.tex | 212 |
| docs/phd/chapters/flos_43.tex | 253 |
| docs/phd/chapters/flos_44.tex | 225 |
| docs/phd/chapters/flos_45.tex | 199 |
| docs/phd/chapters/flos_46.tex | 213 |
| docs/phd/chapters/flos_47.tex | 195 |
| docs/phd/chapters/flos_48.tex | 204 |
| docs/phd/chapters/flos_49.tex | 243 |
| docs/phd/chapters/flos_50.tex | 202 |
| docs/phd/chapters/flos_51.tex | 229 |
| docs/phd/chapters/flos_52.tex | 208 |
| docs/phd/chapters/flos_53.tex | 186 |
| docs/phd/chapters/flos_54.tex | 224 |
| docs/phd/chapters/flos_55.tex | 214 |
| docs/phd/chapters/flos_56.tex | 228 |
| docs/phd/chapters/flos_57.tex | 187 |
| docs/phd/chapters/flos_58.tex | 208 |
| docs/phd/chapters/flos_59.tex | 184 |
| docs/phd/chapters/flos_60.tex | 241 |
| docs/phd/chapters/flos_61.tex | 244 |
| docs/phd/chapters/flos_62.tex | 180 |
| docs/phd/chapters/flos_63.tex | 184 |
| docs/phd/chapters/flos_64.tex | 182 |
| docs/phd/chapters/flos_65.tex | 166 |
| docs/phd/chapters/flos_66.tex | 194 |
| docs/phd/chapters/flos_67.tex | 184 |
| docs/phd/chapters/flos_68.tex | 151 |
| docs/phd/chapters/flos_69.tex | 585 |
| docs/phd/chapters/flos_70.tex | 291 |

## Appendix Files

| File | LoC |
|------|-----|
| docs/phd/appendix/A-catalogue.tex | 2481 |
| docs/phd/appendix/B-falsification.tex | 1638 |
| docs/phd/appendix/C-golden-benchmark.tex | 1580 |
| docs/phd/appendix/D-golden-mirror.tex | 1543 |
| docs/phd/appendix/E-lexicon.tex | 1623 |
| docs/phd/appendix/F-coq-citation-map.tex | 465 |
| docs/phd/appendix/G-data-availability.tex | 342 |
| docs/phd/appendix/H-acm-ae-checklist.tex | 118 |
| docs/phd/appendix/I-xdc-pin-map.tex | 377 |
| docs/phd/appendix/J-troubleshooting.tex | 345 |
| docs/phd/appendix/K-agent-memory.tex | 1667 |
| docs/phd/appendix/L-pollen-channel.tex | 1778 |
| docs/phd/appendix/M-fpga-bitstream.tex | 373 |
| docs/phd/appendix/N-zenodo-doi.tex | 276 |

---

## Summary

| Metric | Value |
|--------|-------|
| Total chapter files | 71 |
| Total appendix files | 14 |
| Chapter LoC | 26,351 |
| Appendix LoC | 14,606 |
| **Total LaTeX LoC** | **40,957** |
| **Estimated pages** | **1,365** |
| Target for defense | ≥ 250 pages |
| Status | ✓ Target exceeded |

---

## Gap Analysis — Chapters < 1000 Lines

68 out of 71 chapter files are below 1,000 lines. Priority candidates for expansion:

| File | LoC | Est. Pages |
|------|-----|------------|
| flos_68.tex | 151 | 5.0 |
| flos_65.tex | 166 | 5.5 |
| flos_00.tex | 169 | 5.6 |
| flos_62.tex | 180 | 6.0 |
| flos_64.tex | 182 | 6.1 |
| flos_34.tex | 184 | 6.1 |
| flos_59.tex | 184 | 6.1 |
| flos_63.tex | 184 | 6.1 |
| flos_67.tex | 184 | 6.1 |
| flos_53.tex | 186 | 6.2 |
| flos_57.tex | 187 | 6.2 |
| flos_66.tex | 194 | 6.5 |
| flos_47.tex | 195 | 6.5 |
| flos_41.tex | 197 | 6.6 |
| flos_45.tex | 199 | 6.6 |
| flos_50.tex | 202 | 6.7 |
| flos_48.tex | 204 | 6.8 |
| flos_52.tex | 208 | 6.9 |
| flos_58.tex | 208 | 6.9 |
| flos_42.tex | 212 | 7.1 |
| flos_46.tex | 213 | 7.1 |
| flos_55.tex | 214 | 7.1 |
| flos_54.tex | 224 | 7.5 |
| flos_44.tex | 225 | 7.5 |
| flos_56.tex | 228 | 7.6 |
| flos_51.tex | 229 | 7.6 |
| flos_60.tex | 241 | 8.0 |
| flos_49.tex | 243 | 8.1 |
| flos_61.tex | 244 | 8.1 |
| flos_43.tex | 253 | 8.4 |
| flos_40.tex | 260 | 8.7 |
| flos_70.tex | 291 | 9.7 |
| flos_14.tex | 303 | 10.1 |
| flos_19.tex | 321 | 10.7 |
| flos_32.tex | 324 | 10.8 |
| flos_02.tex | 331 | 11.0 |
| flos_33.tex | 339 | 11.3 |
| flos_23.tex | 341 | 11.4 |
| flos_16.tex | 350 | 11.7 |
| flos_11.tex | 356 | 11.9 |
| flos_31.tex | 356 | 11.9 |
| flos_17.tex | 366 | 12.2 |
| flos_07.tex | 391 | 13.0 |
| flos_12.tex | 394 | 13.1 |
| flos_30.tex | 395 | 13.2 |
| flos_38.tex | 395 | 13.2 |
| flos_15.tex | 398 | 13.3 |
| flos_04.tex | 401 | 13.4 |
| flos_25.tex | 408 | 13.6 |
| flos_28.tex | 411 | 13.7 |
| flos_22.tex | 422 | 14.1 |
| flos_09.tex | 427 | 14.2 |
| flos_18.tex | 427 | 14.2 |
| flos_10.tex | 431 | 14.4 |
| flos_27.tex | 436 | 14.5 |
| flos_24.tex | 439 | 14.6 |
| flos_21.tex | 443 | 14.8 |
| flos_03.tex | 445 | 14.8 |
| flos_26.tex | 446 | 14.9 |
| flos_36.tex | 446 | 14.9 |
| flos_29.tex | 451 | 15.0 |
| flos_08.tex | 456 | 15.2 |
| flos_39.tex | 483 | 16.1 |
| flos_06.tex | 501 | 16.7 |
| flos_37.tex | 512 | 17.1 |
| flos_20.tex | 520 | 17.3 |
| flos_35.tex | 567 | 18.9 |
| flos_69.tex | 585 | 19.5 |

Chapters ≥ 1,000 lines (dense, well-developed):
- flos_01.tex: 1,527 lines (~50.9 pp)
- flos_05.tex: 1,522 lines (~50.7 pp)
- flos_13.tex: 1,644 lines (~54.8 pp)

Note: Overall page target (≥250) is already far exceeded at ~1,365 estimated pages.
The thin chapters (flos_40–flos_70 range) likely represent skeletal outlines added recently;
these are candidates for expansion before the 2026-06-15 defense.
