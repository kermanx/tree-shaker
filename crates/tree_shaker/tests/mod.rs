use insta::{assert_snapshot, glob};
use oxc::{codegen::CodegenOptions, minifier::MinifierOptions};
use std::fs;
use tree_shaker::{TreeShakeConfig, TreeShakeOptions, tree_shake, vfs::SingleFileFs};

fn do_tree_shake(input: String) -> String {
  let do_minify = input.contains("@minify");
  let react_jsx = input.contains("@react-jsx");
  let result = tree_shake(
    TreeShakeOptions {
      vfs: SingleFileFs(input),
      config: TreeShakeConfig::recommended().with_react_jsx(react_jsx),
      minify_options: do_minify.then(|| MinifierOptions { mangle: None, ..Default::default() }),
      codegen_options: CodegenOptions { annotation_comments: true, ..Default::default() },
    },
    SingleFileFs::ENTRY_PATH.to_string(),
  );
  result.codegen_return[SingleFileFs::ENTRY_PATH].code.clone()
}

#[test]
fn test() {
  glob!("fixtures/**/*.js", |path| {
    let input = fs::read_to_string(path).unwrap();
    let mut settings = insta::Settings::clone_current();
    settings.set_prepend_module_to_snapshot(false);
    settings.bind(|| {
      assert_snapshot!(do_tree_shake(input));
    })
  });
}
