//! Guards that every account contract variant still assembles. `naming.masm` is covered
//! indirectly by the behavioural tests, but `naming_unsafe.masm` and `naming_discount.masm` are
//! not instantiated anywhere, so nothing else would catch a syntax or opcode regression in them.

use std::{fs, path::Path, sync::Arc};

use miden_assembly::{
    DefaultSourceManager,
    ast::{Module, ModuleKind},
};
use miden_protocol::transaction::TransactionKernel;
use miden_standards::StandardsLib;

fn assemble(path: &str) {
    let code = fs::read_to_string(Path::new(path)).unwrap();
    let source_manager = Arc::new(DefaultSourceManager::default());
    let assembler = TransactionKernel::assembler_with_source_manager(source_manager.clone())
        .with_dynamic_library(StandardsLib::default())
        .expect("failed to load standards lib");
    let module = Module::parser(ModuleKind::Library)
        .parse_str("naming", code, source_manager)
        .unwrap_or_else(|e| panic!("parse failed for {path}: {e:?}"));
    assembler
        .assemble_library([module])
        .unwrap_or_else(|e| panic!("assembly failed for {path}: {e:?}"));
}

#[test]
fn assembles_naming() {
    assemble("./masm/accounts/naming.masm");
}

#[test]
fn assembles_naming_unsafe() {
    assemble("./masm/accounts/naming_unsafe.masm");
}

#[test]
fn assembles_naming_discount() {
    assemble("./masm/accounts/naming_discount.masm");
}
