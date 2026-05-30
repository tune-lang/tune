#[test]
fn engine_exposes_meta_decl_facts_from_shared_compiler_facts() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_source(
            "app.tn",
            r#"
-- Runs the app.
let run(): String = "ok"
"#,
        )
        .ok_or("source should allocate")?;
    let check = tune.check_source(file).ok_or("source should check")?;
    let run = check.module.items[0].id;

    let facts = tune
        .meta_decl_facts(file, run)
        .map_err(|_| "meta facts should resolve")?;

    assert!(
        facts.facts.iter().any(|fact| {
            matches!(fact, tune_meta::facts::DeclFact::Name(name) if name == "run")
        })
    );
    assert!(facts.facts.iter().any(|fact| {
        matches!(fact, tune_meta::facts::DeclFact::Doc(doc) if doc == "Runs the app.")
    }));
    assert!(
        facts
            .facts
            .iter()
            .any(|fact| matches!(fact, tune_meta::facts::DeclFact::Return(_)))
    );

    Ok(())
}

#[test]
fn engine_exposes_analysis_backed_meta_signature_facts() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_source(
            "app.tn",
            r#"
struct Counter {
  value: Int
}
let make(seed) = Counter {
  value = seed
}
"#,
        )
        .ok_or("source should allocate")?;
    let check = tune.check_source(file).ok_or("source should check")?;
    let make = check.module.items[1].id;

    let facts = tune
        .meta_decl_facts(file, make)
        .map_err(|_| "meta facts should resolve")?;

    assert!(facts.facts.iter().any(|fact| {
        matches!(fact, tune_meta::facts::DeclFact::Return(shape) if shape.nominal_name() == Some("Counter"))
    }));
    assert!(facts.facts.iter().any(|fact| {
        matches!(fact, tune_meta::facts::DeclFact::Params(params)
            if params.len() == 1
                && params[0].name == "seed"
                && params[0].shape == Some(tune_shape::Shape::Int))
    }));

    Ok(())
}

#[test]
fn engine_exposes_analysis_backed_type_schema() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_source(
            "app.tn",
            r#"
struct Box<T> {
  value: T
}
let make(seed): Box<Int> = Box {
  value = seed
}
"#,
        )
        .ok_or("source should allocate")?;
    let check = tune.check_source(file).ok_or("source should check")?;
    let make = check.module.items[1].id;

    let schema = tune
        .meta_decl_type_schema(file, make)
        .map_err(|_| "type schema should resolve")?;

    assert_eq!(schema.params.len(), 1);
    assert_eq!(schema.params[0].name, "seed");
    assert!(matches!(
        schema.params[0].schema,
        tune_meta::type_schema::TypeSchema::Scalar(tune_meta::type_schema::ScalarType::Int)
    ));
    let Some(tune_meta::type_schema::TypeSchema::Nominal { kind, .. }) = schema.ret else {
        return Err("return should be a nominal struct schema");
    };
    let tune_meta::type_schema::NominalKind::Struct { fields, external } = kind else {
        return Err("return should expose struct fields");
    };
    assert!(!external);
    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].name, "value");
    assert!(matches!(
        fields[0].schema,
        tune_meta::type_schema::TypeSchema::Scalar(tune_meta::type_schema::ScalarType::Int)
    ));

    Ok(())
}

struct MetaValueHost;

impl tune_host::Host for MetaValueHost {
    fn modules(&self) -> Vec<tune_host::HostModule> {
        vec![
            tune_host::HostModule::new(
                "meta",
                vec![tune_host::HostFunction::new(
                    "make",
                    Vec::new(),
                    tune_shape::Shape::Struct(tune_shape::NominalShape::external("meta.Pair")),
                )],
            )
            .with_values(vec![tune_host::HostValueType::new(
                "Pair",
                vec![tune_host::HostValueField::new(
                    "count",
                    tune_shape::Shape::Int,
                )],
            )]),
        ]
    }
}

#[test]
fn engine_exposes_host_value_type_schema() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new().with_host(&MetaValueHost);
    let file = tune
        .add_source(
            "app.tn",
            r#"
import "meta".make
let result = make()
"#,
        )
        .ok_or("source should allocate")?;
    let check = tune.check_source(file).ok_or("source should check")?;
    let result = check
        .module
        .items
        .iter()
        .find(|item| item.name.as_deref() == Some("result"))
        .ok_or("result item should exist")?
        .id;

    let schema = tune
        .meta_decl_type_schema(file, result)
        .map_err(|_| "type schema should resolve")?;

    let Some(tune_meta::type_schema::TypeSchema::Nominal { kind, .. }) = schema.ret else {
        return Err("return should be a host value nominal schema");
    };
    let tune_meta::type_schema::NominalKind::Struct { fields, external } = kind else {
        return Err("host value type should expose struct fields");
    };
    assert!(external);
    assert_eq!(fields[0].name, "count");
    assert!(matches!(
        fields[0].schema,
        tune_meta::type_schema::TypeSchema::Scalar(tune_meta::type_schema::ScalarType::Int)
    ));

    Ok(())
}

#[test]
fn engine_exposes_tagged_decls_without_tag_name_special_cases() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_source(
            "app.tn",
            r#"
tag route {}
tag audit {}
@route(path = "/")
let home(): String = "home"
@audit(level = "debug")
let debug(): String = "debug"
"#,
        )
        .ok_or("source should allocate")?;

    let tagged = tune
        .meta_tagged(file, "audit")
        .map_err(|_| "tagged query should resolve")?;

    assert_eq!(tagged.len(), 1);
    assert_eq!(tagged[0].tag.name, "audit");
    assert_eq!(tagged[0].tag.args[0].name.as_deref(), Some("level"));

    Ok(())
}
