use tune_hir::HirId;
use tune_hir::item::{Item, ItemKind};
use tune_hir::module::Module;
use tune_resolve::ResolvedModule;
use tune_shape::{MemberRequirement, NominalShape, Shape};

use crate::type_schema_lower::{lower_shape_expr_for_item, substitute_params};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeclTypeSchema {
    pub decl_id: HirId,
    pub params: Vec<ParamTypeSchema>,
    pub ret: Option<TypeSchema>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParamTypeSchema {
    pub name: String,
    pub schema: TypeSchema,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldTypeSchema {
    pub name: String,
    pub schema: TypeSchema,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariantTypeSchema {
    pub name: String,
    pub payload: Vec<TypeSchema>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScalarType {
    Int,
    Float,
    Size,
    Byte,
    Bool,
    String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NominalKind {
    Struct {
        fields: Vec<FieldTypeSchema>,
        external: bool,
    },
    Enum {
        variants: Vec<VariantTypeSchema>,
    },
    Opaque,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeSchema {
    Hole,
    Never,
    Unit,
    Scalar(ScalarType),
    Param(String),
    Literal(tune_shape::LiteralFact),
    Sequence(Box<TypeSchema>),
    Range(Box<TypeSchema>),
    Tuple(Vec<TypeSchema>),
    Union(Vec<TypeSchema>),
    Optional(Box<TypeSchema>),
    Callable {
        params: Vec<TypeSchema>,
        ret: Box<TypeSchema>,
    },
    Result {
        ok: Box<TypeSchema>,
        err: Box<TypeSchema>,
    },
    Task(Box<TypeSchema>),
    Nominal {
        name: String,
        id: Option<HirId>,
        args: Vec<TypeSchema>,
        kind: NominalKind,
    },
    Structural(Vec<MemberRequirement>),
}

#[must_use]
pub fn decl_type_schema(
    decl_id: HirId,
    facts: &[tune_resolve::CompilerFact],
    analysis: Option<&tune_shape::ShapeAnalysis>,
    module: &Module,
    resolved: &ResolvedModule,
) -> DeclTypeSchema {
    let facts = crate::facts::from_compiler_facts_and_analysis(decl_id, facts, analysis);
    let mut params = Vec::new();
    let mut ret = None;
    for fact in facts.facts {
        match fact {
            crate::facts::DeclFact::Params(param_facts) => {
                params = param_facts
                    .into_iter()
                    .filter_map(|param| {
                        let shape = param.shape?;
                        Some(ParamTypeSchema {
                            name: param.name,
                            schema: shape_type_schema(&shape, module, resolved),
                        })
                    })
                    .collect();
            }
            crate::facts::DeclFact::Return(shape) => {
                ret = Some(shape_type_schema(&shape, module, resolved));
            }
            _ => {}
        }
    }
    if ret.is_none()
        && let Some(analysis) = analysis
    {
        ret = Some(shape_type_schema(
            &analysis.item_current_shape,
            module,
            resolved,
        ));
    }
    DeclTypeSchema {
        decl_id,
        params,
        ret,
    }
}

#[must_use]
pub fn shape_type_schema(shape: &Shape, module: &Module, resolved: &ResolvedModule) -> TypeSchema {
    shape_type_schema_with_subst(shape, module, resolved, &[])
}

fn shape_type_schema_with_subst(
    shape: &Shape,
    module: &Module,
    resolved: &ResolvedModule,
    subst: &[(String, Shape)],
) -> TypeSchema {
    let substituted = substitute_params(shape, subst);
    match substituted {
        Shape::Hole => TypeSchema::Hole,
        Shape::Never => TypeSchema::Never,
        Shape::Unit => TypeSchema::Unit,
        Shape::Int => TypeSchema::Scalar(ScalarType::Int),
        Shape::Float => TypeSchema::Scalar(ScalarType::Float),
        Shape::Size => TypeSchema::Scalar(ScalarType::Size),
        Shape::Byte => TypeSchema::Scalar(ScalarType::Byte),
        Shape::Bool => TypeSchema::Scalar(ScalarType::Bool),
        Shape::String => TypeSchema::Scalar(ScalarType::String),
        Shape::Param(name) => TypeSchema::Param(name),
        Shape::Literal(literal) => TypeSchema::Literal(literal),
        Shape::Sequence(inner) => TypeSchema::Sequence(Box::new(shape_type_schema_with_subst(
            &inner, module, resolved, subst,
        ))),
        Shape::Range(inner) => TypeSchema::Range(Box::new(shape_type_schema_with_subst(
            &inner, module, resolved, subst,
        ))),
        Shape::Tuple(items) => TypeSchema::Tuple(
            items
                .iter()
                .map(|item| shape_type_schema_with_subst(item, module, resolved, subst))
                .collect(),
        ),
        Shape::Union(items) => TypeSchema::Union(
            items
                .iter()
                .map(|item| shape_type_schema_with_subst(item, module, resolved, subst))
                .collect(),
        ),
        Shape::Optional(inner) => TypeSchema::Optional(Box::new(shape_type_schema_with_subst(
            &inner, module, resolved, subst,
        ))),
        Shape::Callable { params, ret } => TypeSchema::Callable {
            params: params
                .iter()
                .map(|param| shape_type_schema_with_subst(param, module, resolved, subst))
                .collect(),
            ret: Box::new(shape_type_schema_with_subst(&ret, module, resolved, subst)),
        },
        Shape::Result { ok, err } => TypeSchema::Result {
            ok: Box::new(shape_type_schema_with_subst(&ok, module, resolved, subst)),
            err: Box::new(shape_type_schema_with_subst(&err, module, resolved, subst)),
        },
        Shape::Task(inner) => TypeSchema::Task(Box::new(shape_type_schema_with_subst(
            &inner, module, resolved, subst,
        ))),
        Shape::Struct(nominal) | Shape::Enum(nominal) => {
            nominal_type_schema(&nominal, &[], module, resolved)
        }
        Shape::Apply { nominal, args } => nominal_type_schema(&nominal, &args, module, resolved),
        Shape::Structural(requirements) => TypeSchema::Structural(requirements),
    }
}

fn nominal_type_schema(
    nominal: &NominalShape,
    args: &[Shape],
    module: &Module,
    resolved: &ResolvedModule,
) -> TypeSchema {
    let item = nominal
        .id
        .and_then(|id| module.items.iter().find(|item| item.id == id));
    let subst = item.map_or_else(Vec::new, |item| type_arg_subst(item, args));
    let kind = item.map_or(NominalKind::Opaque, |item| match item.kind {
        ItemKind::Struct => NominalKind::Struct {
            fields: struct_fields(item, module, resolved, &subst),
            external: item.external.is_some(),
        },
        ItemKind::Enum => NominalKind::Enum {
            variants: enum_variants(item, module, resolved, &subst),
        },
        _ => NominalKind::Opaque,
    });
    TypeSchema::Nominal {
        name: nominal.name.clone(),
        id: nominal.id,
        args: args
            .iter()
            .map(|arg| shape_type_schema(arg, module, resolved))
            .collect(),
        kind,
    }
}

fn struct_fields(
    item: &Item,
    module: &Module,
    resolved: &ResolvedModule,
    subst: &[(String, Shape)],
) -> Vec<FieldTypeSchema> {
    item.fields
        .iter()
        .filter_map(|field| {
            let name = field.name.clone()?;
            let shape = field.shape.as_ref()?;
            let lowered = lower_shape_expr_for_item(shape, item, resolved);
            Some(FieldTypeSchema {
                name,
                schema: shape_type_schema_with_subst(&lowered, module, resolved, subst),
            })
        })
        .collect()
}

fn enum_variants(
    item: &Item,
    module: &Module,
    resolved: &ResolvedModule,
    subst: &[(String, Shape)],
) -> Vec<VariantTypeSchema> {
    item.variants
        .iter()
        .filter_map(|variant| {
            let name = variant.name.clone()?;
            let payload = variant
                .payload
                .iter()
                .map(|shape| {
                    let lowered = lower_shape_expr_for_item(shape, item, resolved);
                    shape_type_schema_with_subst(&lowered, module, resolved, subst)
                })
                .collect();
            Some(VariantTypeSchema { name, payload })
        })
        .collect()
}

fn type_arg_subst(item: &Item, args: &[Shape]) -> Vec<(String, Shape)> {
    item.type_params
        .iter()
        .filter_map(|param| param.name.clone())
        .zip(args.iter().cloned())
        .collect()
}
