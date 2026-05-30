use tune_db::FileId;

use crate::{EngineError, Tune, has_error_diagnostics};

impl Tune {
    pub fn meta_decl_facts(
        &self,
        file: FileId,
        decl_id: tune_hir::HirId,
    ) -> Result<tune_meta::facts::DeclFacts, EngineError> {
        let check = self
            .check_source(file)
            .ok_or(EngineError::FileNotFound(file))?;
        if has_error_diagnostics(&check.diagnostics) {
            return Err(EngineError::Diagnostics(check.diagnostics));
        }
        let analysis = check
            .module
            .items
            .iter()
            .position(|item| item.id == decl_id)
            .and_then(|index| check.shape.get(index));
        Ok(tune_meta::facts::from_compiler_facts_and_analysis(
            decl_id,
            &check.resolved.facts,
            analysis,
        ))
    }

    pub fn meta_decl_type_schema(
        &self,
        file: FileId,
        decl_id: tune_hir::HirId,
    ) -> Result<tune_meta::type_schema::DeclTypeSchema, EngineError> {
        let check = self
            .check_source(file)
            .ok_or(EngineError::FileNotFound(file))?;
        if has_error_diagnostics(&check.diagnostics) {
            return Err(EngineError::Diagnostics(check.diagnostics));
        }
        let analysis = check
            .module
            .items
            .iter()
            .position(|item| item.id == decl_id)
            .and_then(|index| check.shape.get(index));
        Ok(tune_meta::type_schema::decl_type_schema(
            decl_id,
            &check.resolved.facts,
            analysis,
            &check.module,
            &check.resolved,
        ))
    }

    pub fn meta_tagged(
        &self,
        file: FileId,
        tag_name: &str,
    ) -> Result<Vec<tune_meta::tagged::TaggedDecl<tune_resolve::TagFact>>, EngineError> {
        let check = self
            .check_source(file)
            .ok_or(EngineError::FileNotFound(file))?;
        if has_error_diagnostics(&check.diagnostics) {
            return Err(EngineError::Diagnostics(check.diagnostics));
        }
        Ok(tune_meta::tagged::tagged_decls(
            tag_name,
            &check.resolved.facts,
        ))
    }
}
