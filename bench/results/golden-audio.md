## Summary

- Cases: **11/20 passed** key-fact checks (3 runs each; quality judged on run 1)
- Exact match with the ideal output: **7/19** (whitespace-insensitive)
- Input: synthesized speech → whisper-server; rewrite LLM: on
- Mean ASR WER: **19.2%**
- 5 s silence → **rejected (nothing inserted)**
- Paths: 4 LLM, 0 guard/timeout fallbacks, 16 fast/deterministic

| Stage | P50 | P95 | n |
|---|---|---|---|
| ASR (whisper-server) | 203 ms | 255 ms | 60 |
| Rewrite via LLM | 362 ms | 838 ms | 12 |
| Rewrite fast path | 0 ms | 0 ms | 48 |
| ASR + rewrite | 207 ms | 758 ms | 60 |

## Cases

| | Exact | Case | Path | WER | Output | Note |
|---|---|---|---|---|---|---|
| ✅ | = | `g01` self_correction | llm | 20% | Let's push the release to Friday at 11:30 AM PST. | ASR: Let's push the release to Thursday at 3pm, actually no, sorry, make that Friday at 11:30am PST. |
| ✅ | ≠ | `g02` disfluency | llm | 0% | I was thinking we should refactor the auth middleware before we ship V2. | ASR: Um, so basically I was thinking, uh, like maybe we should, you know, refactor the auth middleware before we, uh, ship V2. |
| ❌ | ≠ | `g03` identifiers | fast | 52% | Run ptest-dash-max-fail=1 on test_user_auth.py and check getUserById. | ASR: Run ptest-dash-max-fail=1 on test_user_auth.py and check camel case get user by id. — missing "--maxfail=1" |
| ❌ | ≠ | `g04` latex | fast | 8% | Write the loss function as $-\frac{1}{n} \sum_{i=1}^{n} y_i \log(\hat{y}_i)$. | ASR: Write the loss function as -1 over n times sigma from i equals 1 to n of y sub i log y hat sub i. — missing "\\frac{1}{N}" |
| ✅ | = | `g05` unicode_math | fast | 0% | In technical mode, verify that θᵀx + b ≥ 0 and α = 0.001. | ASR: In technical mode, verify that theta transpose X plus B is greater than or equal to zero and alpha equals 0.001. |
| ✅ | = | `g06` homophones | fast | 0% | Ensure the kernel serializes the byte array into the Redis cache before their worker thread writes there. |  |
| ✅ | = | `g07` spoken_punctuation | fast | 0% | Send a Slack message saying we need a full stop on new features until the period ends. | ASR: Send a Slack message saying we need a full stop on new features until the period ends, period. |
| ❌ | ≠ | `g08` negation | fast | 8% | Do not drop the staging database unless the backup checksum matches SHAD256. | missing "SHA-256" |
| ✅ | ≠ | `g09` list | fast | 14% | Here are three blockers:⏎1. CUDA out of memory on batch size 64.⏎2. Missing foreign key indexes.⏎3. Flaky WebSocket reconnects. | ASR: Here are three blockers: first, CUDA out of memory on batch size 64, second, missing foreign key indexes, and third, flaky WebSocket reconnects. |
| ✅ | = | `g10` url_path | fast | 70% | Fetch https://api.example.com/v1/users?limit=50 and save to C:\temp\out.json. | ASR: fetch https:\api.example.com/v1\users?limit=50 and save to c:\temp\out.json |
| ❌ | ≠ | `g11` numbers | llm | 36% | The 15.65 million row ETL job dropped runtime from 9 minutes to 44 seconds, saving $1,250.75 per month at 1 times 10 to the power of negative fourth error tolerance. | missing "1e-4" |
| ❌ | ≠ | `g12` ml_jargon +dict | fast | 0% | Combine FashionCLIP dense embeddings in FAISS with sparse BM25 scores using reciprocal rank fusion. | missing "Reciprocal Rank Fusion" |
| ❌ | ≠ | `g13` long_pause | fast | 25% | The primary bottleneck isn't CPU throughput, it's PCIe host to device memory transfer latency. | missing "host-to-device" |
| ✅ |  | `g14` noise_only | fast | - |  |  |
| ❌ | ≠ | `g15` quotes | fast | 22% | Reply to Dr. Patel saying, "RR squared reached 0.841 with P < 0.001", and attach the CSV. | ASR: Reply to Dr. Patel saying, "RR squared reached 0.841 with P less than 0.001", and attach the CSV. — missing "\"Our R" |
| ❌ | ≠ | `g16` model_strings | fast | 50% | Benchmark QEN 2.53B Instruct Q4_K_M on an i9-14900HX with an RTX 4068GB GPU. | missing "RTX 4060" |
| ✅ | = | `g17` possessives | fast | 0% | AWS's IAM policy blocked Lambda's access because it's missing its execution role. |  |
| ✅ | = | `g18` scratch_that | llm | 10% | Tell them the migration is paused pending QA sign-off. | ASR: Tell the team the migration is complete, scratch that whole sentence, tell them the migration is paused pending QA sign off. |
| ✅ | ≠ | `g19` complexity | fast | 18% | The worst case time complexity is O(V + E log V) using a binary min heap, while space stays O(V). | ASR: The worst case time complexity is O of V plus E log V using a binary min heap, while space stays O of V. |
| ❌ | ≠ | `g20` names +dict | fast | 31% | Schedule a one-on-one with Nikita Patel and Françoise Lefèvre to review the Namaste Bharat Détay Set Schema. | missing "1-on-1" |
