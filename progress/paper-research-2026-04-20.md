# Paper Research 2026-04-20

## Topic: In-context learning memory augmentation
- **Title**: Oblivion: Self-Adaptive Agentic Memory Control through Decay-Driven Activation
- **Authors**: Ashish Rana, Chia-Chien Hung, Qumeng Sun, Julian Martin Kunkel, Carolin Lawrence
- **Published**: 2026-03-31T18:37:35Z
- **Link**: http://arxiv.org/abs/2604.00131v2
- **Summary**: Human memory adapts through selective forgetting: experiences become less accessible over time but can be reactivated by reinforcement or contextual cues. In contrast, memory-augmented LLM agents rely on "always-on" retrieval and "flat" memory storage, causing high interference and latency as histories grow. We introduce Oblivion, a memory control framework that casts forgetting as decay-driven reductions in accessibility, not explicit deletion. Oblivion decouples memory control into read and write paths. The read path decides when to consult memory, based on agent uncertainty and memory buffer sufficiency, avoiding redundant always-on access. The write path decides what to strengthen, by reinforcing memories contributing to forming the response. Together, this enables hierarchical memory organization that maintains persistent high-level strategies while dynamically loading details as needed. We evaluate on both static and dynamic long-horizon interaction benchmarks. Results show that Oblivion dynamically adapts memory access and reinforcement, balancing learning and forgetting under shifting contexts, highlighting that memory control is essential for effective LLM-agentic reasoning.
- **Core claim / technique**: Decay-driven accessibility reductions (not explicit deletion) for memory control. Decouples read path (consults memory based on agent uncertainty) and write path (reinforces memories that form response).
- **Potential integration point**: `src/` (maybe a new memory control module or augmenting existing `SparseWeights`/reservoir with decay) and CLI for interaction.
- **Estimated impact**: MEDIUM

## Topic: In-context learning memory augmentation
- **Title**: Seeing the Scene Matters: Revealing Forgetting in Video Understanding Models with a Scene-Aware Long-Video Benchmark
- **Authors**: Seng Nam Chen, Hao Chen, Chenglam Ho, Xinyu Mao, Jinping Wang, Yu Zhang, Chao Li
- **Published**: 2026-03-28T12:44:19Z
- **Link**: http://arxiv.org/abs/2603.27259v1
- **Summary**: Long video understanding (LVU) remains a core challenge in multimodal learning. Although recent vision-language models (VLMs) have made notable progress, existing benchmarks mainly focus on either fine-grained perception or coarse summarization, offering limited insight into temporal understanding over long contexts. In this work, we define a scene as a coherent segment of a video in which both visual and semantic contexts remain consistent, aligning with human perception. This leads us to a key question: can current VLMs reason effectively over long, scene-level contexts? To answer this, we introduce a new benchmark, SceneBench, designed to provide scene-level challenges. Our evaluation reveals a sharp drop in accuracy when VLMs attempt to answer scene-level questions, indicating significant forgetting of long-range context. To further validate these findings, we propose Scene Retrieval-Augmented Generation (Scene-RAG), which constructs a dynamic scene memory by retrieving and integrating relevant context across scenes. This Scene-RAG improves VLM performance by +2.50%, confirming that current models still struggle with long-context retention. We hope SceneBench will encourage future research toward VLMs with more robust, human-like video comprehension.
- **Core claim / technique**: Scene Retrieval-Augmented Generation (Scene-RAG) for dynamic scene memory by retrieving context across scenes.
- **Potential integration point**: Video specific.
- **Estimated impact**: LOW

## Topic: In-context learning memory augmentation
- **Title**: The Library Theorem: How External Organization Governs Agentic Reasoning Capacity
- **Authors**: Zachary F. Mainen
- **Published**: 2026-03-22T15:02:56Z
- **Link**: http://arxiv.org/abs/2603.21272v1
- **Summary**: Externalized reasoning is already exploited by transformer-based agents through chain-of-thought, but structured retrieval -- indexing over one's own reasoning state -- remains underexplored. We formalize the transformer context window as an I/O page and prove that tool-augmented agents with indexed external memory achieve exponentially lower retrieval cost than agents restricted to sequential scanning: $O(\log_b N)$ versus $Ω(N)$ page reads per query, and $O(T \log_b T)$ versus $Θ(T^2)$ cumulative cost over $T$ reasoning steps -- a gap that widens as deliberation deepens. We test these predictions on a controlled lookup benchmark across three content types -- random hashes, ordered integers, and encyclopedia entries -- varying store size from 50 to 5,000 items, and replicate key conditions across two model generations (GPT-4o-mini and GPT-5.4). On abstract content, the indexed agent achieves median 1 page read regardless of store size, confirming the $O(1)$ prediction. Sorted pages without an index fail to close the gap: the weaker model cannot sustain binary search at scale, and the stronger model achieves near-optimal $\log_2 N$ search but still loses to the index by $5\times$. On familiar content (encyclopedia entries), a competing failure mode emerges: the model recognizes the domain, bypasses the retrieval protocol, and generates answers from parametric memory, producing catastrophic token expenditure even when the index is sound. This parametric memory competition dissociates the two cognitive operations that indexing combines: understanding content (where language models excel) and following navigational protocols (where they fail when understanding tempts them to shortcut). The result argues for a separation of concerns: use language models for index construction, where semantic understanding helps, and deterministic algorithms for index traversal, where it hurts.
- **Core claim / technique**: Tool-augmented agents with indexed external memory achieve lower retrieval costs. Argues for using deterministic algorithms for index traversal instead of LLMs.
- **Potential integration point**: We already use deterministic retrieval (BM25, HDC).
- **Estimated impact**: LOW

## Topic: In-context learning memory augmentation
- **Title**: Memori: A Persistent Memory Layer for Efficient, Context-Aware LLM Agents
- **Authors**: Luiz C. Borro, Luiz A. B. Macarini, Gordon Tindall, Michael Montero, Adam B. Struck
- **Published**: 2026-03-20T13:26:38Z
- **Link**: http://arxiv.org/abs/2603.19935v1
- **Summary**: As large language models (LLMs) evolve into autonomous agents, persistent memory at the API layer is essential for enabling context-aware behavior across LLMs and multi-session interactions. Existing approaches force vendor lock-in and rely on injecting large volumes of raw conversation into prompts, leading to high token costs and degraded performance.   We introduce Memori, an LLM-agnostic persistent memory layer that treats memory as a data structuring problem. Its Advanced Augmentation pipeline converts unstructured dialogue into compact semantic triples and conversation summaries, enabling precise retrieval and coherent reasoning.   Evaluated on the LoCoMo benchmark, Memori achieves 81.95% accuracy, outperforming existing memory systems while using only 1,294 tokens per query (~5% of full context). This results in substantial cost reductions, including 67% fewer tokens than competing approaches and over 20x savings compared to full-context methods.   These results show that effective memory in LLM agents depends on structured representations instead of larger context windows, enabling scalable and cost-efficient deployment.
- **Core claim / technique**: Converts unstructured dialogue into compact semantic triples and summaries for precise retrieval instead of flat large contexts.
- **Potential integration point**: `src/` (new memory struct for semantic triples) and `benches/` for context reduction evaluations.
- **Estimated impact**: HIGH

## Topic: In-context learning memory augmentation
- **Title**: MemArchitect: A Policy Driven Memory Governance Layer
- **Authors**: Lingavasan Suresh Kumar, Yang Ba, Rong Pan
- **Published**: 2026-03-18T22:37:05Z
- **Link**: http://arxiv.org/abs/2603.18330v1
- **Summary**: Persistent Large Language Model (LLM) agents expose a critical governance gap in memory management. Standard Retrieval-Augmented Generation (RAG) frameworks treat memory as passive storage, lacking mechanisms to resolve contradictions, enforce privacy, or prevent outdated information ("zombie memories") from contaminating the context window.   We introduce MemArchitect, a governance layer that decouples memory lifecycle management from model weights. MemArchitect enforces explicit, rule-based policies, including memory decay, conflict resolution, and privacy controls.   We demonstrate that governed memory consistently outperforms unmanaged memory in agentic settings, highlighting the necessity of structured memory governance for reliable and safe autonomous systems.
- **Core claim / technique**: Decouples memory lifecycle management. Enforces explicit rule-based policies like memory decay, conflict resolution, and privacy controls.
- **Potential integration point**: `src/` (governance layer on top of retrieval) and `tests/` for rule validations.
- **Estimated impact**: HIGH
