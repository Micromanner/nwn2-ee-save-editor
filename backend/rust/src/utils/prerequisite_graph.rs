use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use std::collections::HashMap;
use std::time::Instant;
use rayon::prelude::*;
use parking_lot::RwLock;
use std::sync::Arc;

/// High-performance prerequisite graph for feat validation
/// Replaces Python implementation taking 10s with Rust implementation < 1s
#[pyclass]
#[derive(Clone)]
pub struct PrerequisiteGraph {
    /// Feat_id (index) -> list of all required feat_ids (flattened)
    feat_requirements: Vec<Vec<u32>>,

    /// Feat_id (index) -> direct prerequisites
    direct_prerequisites: Vec<Prerequisites>,

    /// Statistics about the graph
    stats: GraphStats,

    /// Build time in milliseconds
    build_time_ms: f64,

    /// Whether the graph has been built
    is_built: bool,
}

#[derive(Clone, Debug)]
struct Prerequisites {
    feats: Vec<u32>,
    abilities: HashMap<String, u32>,
    class: Option<u32>,
    level: u32,
    bab: u32,
    spell_level: u32,
}

impl Default for Prerequisites {
    fn default() -> Self {
        Prerequisites {
            feats: Vec::new(),
            abilities: HashMap::new(),
            class: None,
            level: 0,
            bab: 0,
            spell_level: 0,
        }
    }
}

#[derive(Clone, Debug)]
struct GraphStats {
    total_feats: usize,
    feats_with_prereqs: usize,
    max_chain_depth: usize,
    circular_dependencies: Vec<u32>,
}

impl Default for GraphStats {
    fn default() -> Self {
        GraphStats {
            total_feats: 0,
            feats_with_prereqs: 0,
            max_chain_depth: 0,
            circular_dependencies: Vec::new(),
        }
    }
}

impl PrerequisiteGraph {
    /// Internal recursive function for flattening prerequisites
    fn flatten_prerequisites_internal(
        feat_id: u32,
        visited: &mut Vec<bool>,
        depth: usize,
        direct_prereqs: &[Prerequisites],
        max_depth: &Arc<RwLock<usize>>,
        circular_deps: &Arc<RwLock<Vec<u32>>>
    ) -> Vec<u32> {
        let idx = feat_id as usize;

        // Check for circular dependency
        if idx >= visited.len() || visited[idx] {
            if idx < visited.len() {
                circular_deps.write().push(feat_id);
            }
            return Vec::new();
        }

        // Update max depth
        {
            let mut max = max_depth.write();
            if depth > *max {
                *max = depth;
            }
        }

        visited[idx] = true;

        // Get direct prerequisites
        let mut all_requirements = Vec::new();
        if idx < direct_prereqs.len() {
            let prereqs = &direct_prereqs[idx];
            for &req_feat in &prereqs.feats {
                if !all_requirements.contains(&req_feat) {
                    all_requirements.push(req_feat);
                }

                // Recursively add prerequisites of prerequisites
                let nested = Self::flatten_prerequisites_internal(
                    req_feat,
                    visited,
                    depth + 1,
                    direct_prereqs,
                    max_depth,
                    circular_deps
                );
                for req in nested {
                    if !all_requirements.contains(&req) {
                        all_requirements.push(req);
                    }
                }
            }
        }

        visited[idx] = false;
        all_requirements
    }
}

#[pymethods]
impl PrerequisiteGraph {
    #[new]
    pub fn new() -> Self {
        PrerequisiteGraph {
            feat_requirements: Vec::new(),
            direct_prerequisites: Vec::new(),
            stats: GraphStats::default(),
            build_time_ms: 0.0,
            is_built: false,
        }
    }
    
    /// Build the graph from feat data
    /// Expected to reduce 10s Python implementation to < 1s
    pub fn build_from_data(&mut self, feat_data: &Bound<'_, PyList>) -> PyResult<()> {
        let start = Instant::now();

        let total_feats = feat_data.len();
        self.stats.total_feats = total_feats;

        // Pre-allocate vectors
        let mut direct_prereqs: Vec<Prerequisites> = vec![Prerequisites::default(); total_feats];

        // First pass: Extract direct prerequisites
        for (index, item) in feat_data.iter().enumerate() {
            let feat_dict: Bound<'_, PyDict> = item.extract()?;

            let mut prereqs = Prerequisites::default();

            // Extract feat prerequisites
            if let Some(feat_list) = feat_dict.get_item("prereqfeat1")? {
                if let Ok(val) = feat_list.extract::<i32>() {
                    if val >= 0 {
                        prereqs.feats.push(val as u32);
                    }
                }
            }
            if let Some(feat_list) = feat_dict.get_item("prereqfeat2")? {
                if let Ok(val) = feat_list.extract::<i32>() {
                    if val >= 0 {
                        prereqs.feats.push(val as u32);
                    }
                }
            }

            // Extract ability requirements
            if let Some(min_str) = feat_dict.get_item("minstr")? {
                if let Ok(val) = min_str.extract::<u32>() {
                    if val > 0 {
                        prereqs.abilities.insert("strength".to_string(), val);
                    }
                }
            }
            if let Some(min_dex) = feat_dict.get_item("mindex")? {
                if let Ok(val) = min_dex.extract::<u32>() {
                    if val > 0 {
                        prereqs.abilities.insert("dexterity".to_string(), val);
                    }
                }
            }
            if let Some(min_con) = feat_dict.get_item("mincon")? {
                if let Ok(val) = min_con.extract::<u32>() {
                    if val > 0 {
                        prereqs.abilities.insert("constitution".to_string(), val);
                    }
                }
            }
            if let Some(min_int) = feat_dict.get_item("minint")? {
                if let Ok(val) = min_int.extract::<u32>() {
                    if val > 0 {
                        prereqs.abilities.insert("intelligence".to_string(), val);
                    }
                }
            }
            if let Some(min_wis) = feat_dict.get_item("minwis")? {
                if let Ok(val) = min_wis.extract::<u32>() {
                    if val > 0 {
                        prereqs.abilities.insert("wisdom".to_string(), val);
                    }
                }
            }
            if let Some(min_cha) = feat_dict.get_item("mincha")? {
                if let Ok(val) = min_cha.extract::<u32>() {
                    if val > 0 {
                        prereqs.abilities.insert("charisma".to_string(), val);
                    }
                }
            }

            // Extract other requirements
            if let Some(min_level) = feat_dict.get_item("minlevel")? {
                if let Ok(val) = min_level.extract::<u32>() {
                    prereqs.level = val;
                }
            }
            if let Some(min_bab) = feat_dict.get_item("minattackbonus")? {
                if let Ok(val) = min_bab.extract::<u32>() {
                    prereqs.bab = val;
                }
            }
            if let Some(spell_level) = feat_dict.get_item("minspelllvl")? {
                if let Ok(val) = spell_level.extract::<u32>() {
                    prereqs.spell_level = val;
                }
            }

            // Check if has any prerequisites
            let has_prereqs = !prereqs.feats.is_empty()
                || !prereqs.abilities.is_empty()
                || prereqs.class.is_some()
                || prereqs.level > 0
                || prereqs.bab > 0
                || prereqs.spell_level > 0;

            if has_prereqs {
                self.stats.feats_with_prereqs += 1;
            }

            direct_prereqs[index] = prereqs;
        }

        self.direct_prerequisites = direct_prereqs.clone();

        // Second pass: Flatten prerequisite chains
        let max_depth = Arc::new(RwLock::new(0usize));
        let circular_deps = Arc::new(RwLock::new(Vec::new()));

        // Process all feats in parallel using Rayon
        let flattened_results: Vec<Vec<u32>> = (0..total_feats)
            .into_par_iter()
            .map(|feat_id| {
                let mut visited = vec![false; total_feats];
                Self::flatten_prerequisites_internal(
                    feat_id as u32,
                    &mut visited,
                    1,
                    &direct_prereqs,
                    &max_depth,
                    &circular_deps
                )
            })
            .collect();

        self.feat_requirements = flattened_results;
        self.stats.max_chain_depth = *max_depth.read();
        self.stats.circular_dependencies = circular_deps.read().clone();
        
        self.build_time_ms = start.elapsed().as_millis() as f64;
        self.is_built = true;
        
        Ok(())
    }
    
    /// Get all flattened feat requirements for a given feat (O(1) lookup)
    pub fn get_all_feat_requirements(&self, feat_id: u32) -> PyResult<Vec<u32>> {
        if !self.is_built {
            return Ok(Vec::new());
        }

        let idx = feat_id as usize;
        if idx < self.feat_requirements.len() {
            Ok(self.feat_requirements[idx].clone())
        } else {
            Ok(Vec::new())
        }
    }
    
    /// Get direct prerequisites for a feat
    pub fn get_direct_prerequisites(&self, py: Python<'_>, feat_id: u32) -> PyResult<Py<PyAny>> {
        let dict = PyDict::new(py);
        let idx = feat_id as usize;

        if idx < self.direct_prerequisites.len() {
            let prereqs = &self.direct_prerequisites[idx];
            dict.set_item("feats", &prereqs.feats)?;

            let abilities_dict = PyDict::new(py);
            for (ability, value) in &prereqs.abilities {
                abilities_dict.set_item(ability, value)?;
            }
            dict.set_item("abilities", abilities_dict)?;
            dict.set_item("class", prereqs.class)?;
            dict.set_item("level", prereqs.level)?;
            dict.set_item("bab", prereqs.bab)?;
            dict.set_item("spell_level", prereqs.spell_level)?;
        } else {
            dict.set_item("feats", Vec::<u32>::new())?;
            dict.set_item("abilities", PyDict::new(py))?;
            dict.set_item("class", Option::<u32>::None)?;
            dict.set_item("level", 0)?;
            dict.set_item("bab", 0)?;
            dict.set_item("spell_level", 0)?;
        }

        Ok(dict.into())
    }
    
    /// Fast validation using pre-computed graph
    pub fn validate_feat_prerequisites_fast(
        &self,
        feat_id: u32,
        character_feats: &Bound<'_, PyAny>,
        character_data: Option<&Bound<'_, PyDict>>
    ) -> PyResult<(bool, Vec<String>)> {
        if !self.is_built {
            return Ok((true, Vec::new()));
        }

        let mut errors = Vec::new();

        // Build Vec<bool> for character feats
        let mut char_has_feat = vec![false; self.feat_requirements.len()];
        for item in character_feats.try_iter()? {
            if let Ok(feat) = item?.extract::<u32>() {
                let idx = feat as usize;
                if idx < char_has_feat.len() {
                    char_has_feat[idx] = true;
                }
            }
        }

        // Check feat requirements using pre-computed graph
        let idx = feat_id as usize;
        if idx < self.feat_requirements.len() {
            for &req_feat in &self.feat_requirements[idx] {
                let req_idx = req_feat as usize;
                if req_idx >= char_has_feat.len() || !char_has_feat[req_idx] {
                    errors.push(format!("Requires Feat {}", req_feat));
                }
            }
        }

        // Check other requirements if character data provided
        if let Some(data) = character_data {
            if idx < self.direct_prerequisites.len() {
                let prereqs = &self.direct_prerequisites[idx];

                // Check ability requirements
                for (ability, min_score) in &prereqs.abilities {
                    if let Some(current) = data.get_item(ability)? {
                        if let Ok(val) = current.extract::<u32>() {
                            if val < *min_score {
                                errors.push(format!("Requires {} {}", ability.to_uppercase(), min_score));
                            }
                        }
                    }
                }

                // Check level requirement
                if prereqs.level > 0 {
                    if let Some(level) = data.get_item("level")? {
                        if let Ok(val) = level.extract::<u32>() {
                            if val < prereqs.level {
                                errors.push(format!("Requires character level {}", prereqs.level));
                            }
                        }
                    }
                }

                // Check BAB requirement
                if prereqs.bab > 0 {
                    if let Some(bab) = data.get_item("bab")? {
                        if let Ok(val) = bab.extract::<u32>() {
                            if val < prereqs.bab {
                                errors.push(format!("Requires base attack bonus +{}", prereqs.bab));
                            }
                        }
                    }
                }
            }
        }

        Ok((errors.is_empty(), errors))
    }

    /// Validate multiple feats at once (batch operation)
    pub fn validate_batch_fast(
        &self,
        feat_ids: Vec<u32>,
        character_feats: &Bound<'_, PyAny>,
        character_data: Option<&Bound<'_, PyDict>>
    ) -> PyResult<HashMap<u32, (bool, Vec<String>)>> {
        if !self.is_built {
            return Ok(HashMap::new());
        }

        // Build Vec<bool> once, reuse for all validations
        let mut char_has_feat = vec![false; self.feat_requirements.len()];
        for item in character_feats.try_iter()? {
            if let Ok(feat) = item?.extract::<u32>() {
                let idx = feat as usize;
                if idx < char_has_feat.len() {
                    char_has_feat[idx] = true;
                }
            }
        }

        let mut results = HashMap::new();

        for feat_id in feat_ids {
            let mut errors = Vec::new();
            let idx = feat_id as usize;

            // Check feat requirements
            if idx < self.feat_requirements.len() {
                for &required in &self.feat_requirements[idx] {
                    let req_idx = required as usize;
                    if req_idx >= char_has_feat.len() || !char_has_feat[req_idx] {
                        errors.push(format!("Requires Feat {}", required));
                    }
                }
            }

            // Check other requirements if character data provided
            if let Some(data) = character_data {
                if idx < self.direct_prerequisites.len() {
                    let prereqs = &self.direct_prerequisites[idx];

                    for (ability, min_score) in &prereqs.abilities {
                        if let Some(current) = data.get_item(ability)? {
                            if let Ok(val) = current.extract::<u32>() {
                                if val < *min_score {
                                    errors.push(format!("Requires {} {}", ability.to_uppercase(), min_score));
                                }
                            }
                        }
                    }

                    if prereqs.level > 0 {
                        if let Some(level) = data.get_item("level")? {
                            if let Ok(val) = level.extract::<u32>() {
                                if val < prereqs.level {
                                    errors.push(format!("Requires character level {}", prereqs.level));
                                }
                            }
                        }
                    }

                    if prereqs.bab > 0 {
                        if let Some(bab) = data.get_item("bab")? {
                            if let Ok(val) = bab.extract::<u32>() {
                                if val < prereqs.bab {
                                    errors.push(format!("Requires base attack bonus +{}", prereqs.bab));
                                }
                            }
                        }
                    }
                }
            }

            results.insert(feat_id, (errors.is_empty(), errors));
        }

        Ok(results)
    }
    
    /// Get statistics about the graph
    pub fn get_statistics(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let dict = PyDict::new(py);
        dict.set_item("is_built", self.is_built)?;
        dict.set_item("build_time_ms", self.build_time_ms)?;
        dict.set_item("total_feats", self.stats.total_feats)?;
        dict.set_item("feats_with_prerequisites", self.stats.feats_with_prereqs)?;
        dict.set_item("max_chain_depth", self.stats.max_chain_depth)?;
        dict.set_item("circular_dependencies_count", self.stats.circular_dependencies.len())?;
        dict.set_item("memory_estimate_mb",
            (self.feat_requirements.len() * 100) as f64 / (1024.0 * 1024.0))?;
        Ok(dict.into())
    }
}