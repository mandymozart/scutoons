# wiring (#393, pc): median of 3 rounds of 5000 blocks; mean µs per 128-frame block (budget 2667 µs), p99 in brackets

| stage | K | wasmtime | wired modules | merged in one | native | wired − merged | per crossing | of budget | wired ÷ merged | bits equal |
|---|---|---|---|---|---|---|---|---|---|---|
| light | 2 | deck | 8.2 (11.2) | 7.9 (11.9) | 3.4 (4.4) | 0.3 | 0.32 | 0.0 % | 1.04× | yes |
| light | 2 | tuned | 4.3 (6.4) | 3.7 (4.6) | 3.4 (4.4) | 0.6 | 0.57 | 0.0 % | 1.16× | yes |
| light | 8 | deck | 33.7 (47.4) | 30.3 (44.8) | 13.2 (17.2) | 3.3 | 0.48 | 0.1 % | 1.11× | yes |
| light | 8 | tuned | 17.5 (23.4) | 14.2 (19.5) | 13.2 (17.2) | 3.3 | 0.47 | 0.1 % | 1.23× | yes |
| light | 16 | deck | 66.2 (86.3) | 58.3 (70.3) | 26.9 (34.4) | 7.9 | 0.53 | 0.3 % | 1.14× | yes |
| light | 16 | tuned | 34.8 (43.6) | 27.1 (33.2) | 26.9 (34.4) | 7.7 | 0.51 | 0.3 % | 1.28× | yes |
| heavy | 2 | deck | 39.7 (63.5) | 43.2 (67.0) | 4.5 (6.7) | -3.4 | -3.44 | -0.1 % | 0.92× | yes |
| heavy | 2 | tuned | 11.3 (18.2) | 11.2 (18.9) | 4.5 (6.7) | 0.1 | 0.06 | 0.0 % | 1.01× | yes |
| heavy | 8 | deck | 184.3 (261.6) | 182.8 (250.1) | 17.6 (24.2) | 1.5 | 0.21 | 0.1 % | 1.01× | yes |
| heavy | 8 | tuned | 55.3 (96.6) | 48.8 (75.5) | 17.6 (24.2) | 6.5 | 0.93 | 0.2 % | 1.13× | yes |
| heavy | 16 | deck | 384.0 (543.5) | 376.9 (507.0) | 35.5 (47.7) | 7.1 | 0.47 | 0.3 % | 1.02× | yes |
| heavy | 16 | tuned | 110.0 (158.9) | 94.3 (141.8) | 35.5 (47.7) | 15.7 | 1.05 | 0.6 % | 1.17× | yes |

`per crossing` = (wired − merged) / (K − 1). `wired` has no tempo or event staging; a real module node costs a little more.

## Native nodes through the engine: sixteen FM voices (batches by level)

| wiring | mean µs | p99 µs | of budget |
|---|---|---|---|
| all 16 on the sequencer: one level, one batch | 17.9 | 26.7 | 0.7 % |
| 12 free + 4 sequenced: two levels, two batches | 18.5 | 24.9 | 0.7 % |
| all 16 on the sequencer, patcher build: no batch | 58.5 | 88.5 | 2.2 % |
