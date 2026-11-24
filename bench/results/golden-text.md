## Summary

- Cases: **19/19 passed** key-fact checks (3 runs each; quality judged on run 1)
- Exact match with the ideal output: **16/19** (whitespace-insensitive)
- Input: reference text (ASR bypassed); rewrite LLM: on
- Paths: 4 LLM, 1 guard/timeout fallbacks, 14 fast/deterministic

| Stage | P50 | P95 | n |
|---|---|---|---|
| Rewrite via LLM | 362 ms | 638 ms | 12 |
| Rewrite fast path | 0 ms | 443 ms | 45 |
| ASR + rewrite | 0 ms | 631 ms | 57 |

## Cases

| | Exact | Case | Path | WER | Output | Note |
|---|---|---|---|---|---|---|
| ✅ | = | `g01` self_correction | llm | - | Let's push the release to Friday at 11:30 AM PST. |  |
| ✅ | = | `g02` disfluency | llm | - | I was thinking we should refactor the auth middleware before we ship v2. |  |
| ✅ | = | `g03` identifiers | fast | - | Run pytest --maxfail=1 on test_user_auth.py and check getUserById. |  |
| ✅ | ≠ | `g04` latex | llm | - | Write the loss function as $-\frac{1}{N} \sum_{i=1}^{N} y_i \log(\hat{y}_i)$. |  |
| ✅ | = | `g05` unicode_math | fast | - | In technical mode, verify that θᵀx + b ≥ 0 and α = 0.001. |  |
| ✅ | = | `g06` homophones | fast | - | Ensure the kernel serializes the byte array into the Redis cache before their worker thread writes there. |  |
| ✅ | = | `g07` spoken_punctuation | fast | - | Send a Slack message saying we need a full stop on new features until the period ends. |  |
| ✅ | = | `g08` negation | fast | - | Do not drop the staging database unless the backup checksum matches SHA-256. |  |
| ✅ | = | `g09` list | fast | - | Here are three blockers:⏎1. CUDA out-of-memory on batch size 64.⏎2. Missing foreign key indexes.⏎3. Flaky WebSocket reconnects. |  |
| ✅ | = | `g10` url_path | fallback: guard: dropped identifier "api.example.c | - | Fetch https://api.example.com/v1/users?limit=50 and save to C:\temp\out.json. |  |
| ✅ | = | `g11` numbers | fast | - | The 15.65 million row ETL job dropped runtime from 9 minutes to 44 seconds, saving $1,250.75 per month at 1e-4 error tolerance. |  |
| ✅ | = | `g12` ml_jargon +dict | fast | - | Combine FashionCLIP dense embeddings in FAISS with sparse BM25 scores using Reciprocal Rank Fusion. |  |
| ✅ | ≠ | `g13` long_pause | fast | - | The primary bottleneck isn't CPU throughput it's PCIe host-to-device memory transfer latency. |  |
| ✅ | = | `g15` quotes | fast | - | Reply to Dr. Patel saying, "Our R² reached 0.841 with p < 0.001," and attach the CSV. |  |
| ✅ | ≠ | `g16` model_strings | fast | - | Benchmark Qwen 2.5 3B Instruct Q4_K_M on an i9-14900HX with an RTX 4060 8GB GPU. |  |
| ✅ | = | `g17` possessives | fast | - | AWS's IAM policy blocked Lambda's access because it's missing its execution role. |  |
| ✅ | = | `g18` scratch_that | llm | - | Tell them the migration is paused pending QA sign-off. |  |
| ✅ | = | `g19` complexity | fast | - | The worst-case time complexity is O(V + E log V) using a binary min-heap, while space stays O(V). |  |
| ✅ | = | `g20` names +dict | fast | - | Schedule a 1-on-1 with Niketa Patel and François Lefèvre to review the Namaste Bharat dataset schema. |  |
