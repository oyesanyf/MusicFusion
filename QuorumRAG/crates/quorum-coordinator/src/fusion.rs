use quorum_types::{CandidateChunkDto, DocumentChunk, ScoredChunk};
use std::collections::HashMap;

pub struct RrfCombiner;

impl RrfCombiner {
    /// Computes theoretical RRF score across 1-based ranks: sum 1 / (k + r).
    pub fn compute_rrf_score(ranks: &[usize], k: f32) -> f32 {
        ranks.iter().map(|&r| 1.0 / (k + r as f32)).sum()
    }

    /// Combines local rankings and remote peer candidate lists using Reciprocal Rank Fusion ($k=60.0$)
    /// with BLAKE3 deduplication and deterministic tie-breaking on `(chunk_id, blake3_hash)`.
    pub fn fuse_rankings(
        local_results: Vec<ScoredChunk>,
        remote_peer_lists: Vec<Vec<CandidateChunkDto>>,
        k: f32,
        top_k: usize,
    ) -> Vec<ScoredChunk> {
        let mut rrf_map: HashMap<String, (f32, ScoredChunk)> = HashMap::new();

        // 1. Local chunk rankings
        for (rank_0, scored) in local_results.into_iter().enumerate() {
            let score = 1.0 / (k + (rank_0 + 1) as f32);
            let entry = rrf_map
                .entry(scored.chunk.chunk_id.clone())
                .or_insert((0.0, scored));
            entry.0 += score;
        }

        // 2. Remote peer chunk rankings
        for list in remote_peer_lists {
            for (rank_0, candidate) in list.into_iter().enumerate() {
                let score = 1.0 / (k + (rank_0 + 1) as f32);
                let doc_chunk = DocumentChunk {
                    chunk_id: candidate.chunk_id.clone(),
                    document_id: "remote_doc".to_string(),
                    file_path: candidate.file_path,
                    line_start: candidate.line_start,
                    line_end: candidate.line_end,
                    blake3_hash: DocumentChunk::compute_blake3_hash(&candidate.content),
                    access_tier: candidate.access_tier,
                    vector: vec![0.0; 768],
                    text_content: candidate.content,
                    metadata_json: "{}".to_string(),
                };
                let scored = ScoredChunk::new(doc_chunk, candidate.score, "remote_peer", true);
                let entry = rrf_map.entry(candidate.chunk_id).or_insert((0.0, scored));
                entry.0 += score;
            }
        }

        // 3. Finalize scores
        let mut fused: Vec<ScoredChunk> = rrf_map
            .into_iter()
            .map(|(_id, (rrf_score, mut chunk))| {
                chunk.score = rrf_score;
                chunk
            })
            .collect();

        // 4. Deterministic tie-breaking
        fused.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.chunk.chunk_id.cmp(&b.chunk.chunk_id))
                .then_with(|| a.chunk.blake3_hash.cmp(&b.chunk.blake3_hash))
        });

        // 5. Truncate to top_k
        fused.truncate(top_k);
        fused
    }
}
