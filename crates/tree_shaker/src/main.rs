use clap::Parser;
use flate2::{Compression, write::GzEncoder};
use oxc::{
  codegen::CodegenOptions,
  minifier::{MangleOptions, MinifierOptions},
};
use std::{fs::File, io::Write, path::PathBuf};
use tree_shaker::{
  TreeShakeConfig, TreeShakeOptions, tree_shake,
  vfs::{SingleFileFs, StdFs, Vfs},
};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
  path: String,

  #[arg(short, long, default_value_t = false)]
  single_file: bool,

  #[arg(short, long)]
  output: Option<String>,

  #[arg(short, long, default_value_t = false)]
  no_shake: bool,

  #[arg(short, long, default_value_t = false)]
  minify: bool,

  #[arg(short, long, default_value_t = String::from("recommended"))]
  preset: String,

  #[arg(short, long, default_value_t = false)]
  always_inline_literal: bool,

  #[arg(short, long, default_value_t = true)]
  jsx: bool,

  #[arg(long, default_value_t = String::from("on"))]
  // on/off/only
  mangle: String,

  #[arg(short, long, default_value_t = 2)]
  recursion_depth: usize,
}

fn main() {
  let args = Args::parse();

  let shake_disabled = TreeShakeConfig::disabled().with_react_jsx(args.jsx);
  let shake_enabled = match args.preset.as_str() {
    "safest" => TreeShakeConfig::safest(),
    "recommended" => TreeShakeConfig::recommended(),
    "smallest" => TreeShakeConfig::smallest(),
    _ => {
      eprintln!("Invalid preset: {}", args.preset);
      std::process::exit(1);
    }
  }
  .with_react_jsx(args.jsx)
  .with_always_inline_literal(args.always_inline_literal)
  .with_mangling(match args.mangle.as_str() {
    "on" => Some(false),
    "off" => None,
    "only" => Some(true),
    _ => {
      eprintln!("Invalid --mangle: {}", args.mangle);
      std::process::exit(1);
    }
  })
  .with_max_recursion_depth(args.recursion_depth);

  let minify_options = MinifierOptions {
    mangle: Some(MangleOptions { top_level: true, ..Default::default() }),
    ..Default::default()
  };
  let min_codegen_options = CodegenOptions { minify: true, comments: false, ..Default::default() };

  if args.single_file {
    let source = match std::fs::read_to_string(&args.path) {
      Err(why) => {
        eprintln!("Couldn't read {}: {}", args.path, why);
        std::process::exit(1);
      }
      Ok(content) => content,
    };

    let start_time = std::time::Instant::now();

    let copied = tree_shake(
      TreeShakeOptions {
        vfs: SingleFileFs(source.clone()),
        config: shake_disabled.clone(),
        minify_options: None,
        codegen_options: CodegenOptions::default(),
      },
      SingleFileFs::ENTRY_PATH.to_string(),
    );
    let copied_code = copied.codegen_return[SingleFileFs::ENTRY_PATH].code.clone();
    let minified = tree_shake(
      TreeShakeOptions {
        vfs: SingleFileFs(source.clone()),
        config: shake_disabled.clone(),
        minify_options: Some(minify_options),
        codegen_options: min_codegen_options.clone(),
      },
      SingleFileFs::ENTRY_PATH.to_string(),
    );
    let minified_code = minified.codegen_return[SingleFileFs::ENTRY_PATH].code.clone();
    let shaken = tree_shake(
      TreeShakeOptions {
        vfs: SingleFileFs(source.clone()),
        config: shake_enabled,
        minify_options: None,
        codegen_options: CodegenOptions::default(),
      },
      SingleFileFs::ENTRY_PATH.to_string(),
    );
    let shaken_code = shaken.codegen_return[SingleFileFs::ENTRY_PATH].code.clone();
    let shaken_minified = tree_shake(
      TreeShakeOptions {
        vfs: SingleFileFs(shaken_code.clone()),
        config: shake_disabled.clone(),
        minify_options: Some(minify_options),
        codegen_options: min_codegen_options,
      },
      SingleFileFs::ENTRY_PATH.to_string(),
    );
    let shaken_minified_code =
      shaken_minified.codegen_return[SingleFileFs::ENTRY_PATH].code.clone();

    let elapsed = start_time.elapsed();

    for diagnostic in shaken.diagnostics.iter() {
      eprintln!("{}", diagnostic);
    }

    eprintln!("Completed in {:?}", elapsed);
    eprintln!("Original: {}B", copied_code.len());
    eprintln!("Minified: {}B", minified_code.len());
    eprintln!("  Shaken: {}B", shaken_code.len());
    eprintln!("    Both: {}B", shaken_minified_code.len());
    eprintln!(
      "Minified/Both = {:.2}%",
      (shaken_minified_code.len() as f64 / minified_code.len() as f64) * 100.0
    );

    // If the input file is dir/a.js, the output file will be dir/a.out.js
    let output_path = args.output.map_or_else(
      || {
        let mut output_path = PathBuf::from(&args.path);
        if !args.no_shake {
          output_path.set_extension("out.js");
        }
        if args.minify {
          output_path.set_extension("min.js");
        }
        if args.no_shake && !args.minify {
          output_path.set_extension("copy.js");
        }
        output_path
      },
      PathBuf::from,
    );

    let mut output_file = match File::create(&output_path) {
      Err(why) => {
        eprintln!("Couldn't create {}: {}", output_path.display(), why);
        std::process::exit(1);
      }
      Ok(file) => file,
    };

    let code = match (!args.no_shake, args.minify) {
      (true, true) => shaken_minified_code,
      (true, false) => shaken_code,
      (false, true) => minified_code,
      (false, false) => copied_code,
    };
    match output_file.write_all(code.as_bytes()) {
      Err(why) => {
        eprintln!("Couldn't write to {}: {}", output_path.display(), why);
        std::process::exit(1);
      }
      Ok(_) => {
        println!("Wrote to {}", output_path.display());
      }
    }
  } else {
    let start_time = std::time::Instant::now();

    let shaken = tree_shake(
      TreeShakeOptions {
        vfs: StdFs,
        config: shake_enabled,
        minify_options: args.minify.then_some(minify_options),
        codegen_options: if args.minify {
          min_codegen_options.clone()
        } else {
          CodegenOptions::default()
        },
      },
      args.path.clone(),
    );

    for diagnostic in shaken.diagnostics.iter() {
      eprintln!("{}", diagnostic);
    }

    let out_dir = PathBuf::from(args.output.unwrap_or(String::from("output")));
    let _ = std::fs::remove_dir_all(&out_dir);
    if let Err(e) = std::fs::create_dir_all(&out_dir) {
      eprintln!("Couldn't create directory {}: {}", out_dir.display(), e);
      std::process::exit(1);
    }

    let mut input_total = 0;
    let mut output_total = 0;
    let mut input_g_total = 0;
    let mut output_g_total = 0;
    for (path, codegen_return) in shaken.codegen_return {
      let out_path = out_dir.join(&path);
      let orig_ext = if let Some(ext) = out_path.extension() {
        format!("orig.{}", ext.to_string_lossy())
      } else {
        "orig".to_string()
      };
      let copy_path = out_path.with_extension(orig_ext);
      println!("{}\t--> {}", path, out_path.display());

      if let Err(e) = std::fs::create_dir_all(out_path.parent().unwrap()) {
        eprintln!("Couldn't create directory {}: {}", out_dir.display(), e);
        std::process::exit(1);
      }
      let mut output_file = match File::create(&out_path) {
        Err(why) => {
          eprintln!("Couldn't create {}: {}", out_path.display(), why);
          std::process::exit(1);
        }
        Ok(file) => file,
      };
      output_file.write_all(codegen_return.code.as_bytes()).unwrap();

      let source = StdFs.read_file(&path);

      let non_shaken = tree_shake(
        TreeShakeOptions {
          vfs: SingleFileFs(source.clone()),
          config: shake_disabled.clone(),
          minify_options: args.minify.then_some(minify_options),
          codegen_options: if args.minify {
            min_codegen_options.clone()
          } else {
            CodegenOptions::default()
          },
        },
        SingleFileFs::ENTRY_PATH.to_string(),
      );
      let non_shaken_code = non_shaken.codegen_return[SingleFileFs::ENTRY_PATH].code.clone();

      let mut copy_file = match File::create(&copy_path) {
        Err(why) => {
          eprintln!("Couldn't create {}: {}", copy_path.display(), why);
          std::process::exit(1);
        }
        Ok(file) => file,
      };
      copy_file.write_all(non_shaken_code.as_bytes()).unwrap();

      input_total += non_shaken_code.len();
      output_total += codegen_return.code.len();
      println!(
        "    [RAW]  Original: {}B\t{}: {}B\t{}: {}B\tRate: {:.2}%",
        source.len(),
        if args.minify { "Shaken&Minified" } else { "Shaken" },
        codegen_return.code.len(),
        if args.minify { "Minified" } else { "Copied" },
        non_shaken_code.len(),
        (codegen_return.code.len() as f64 / non_shaken_code.len() as f64) * 100.0
      );

      let source_g_size = get_gzipped_size(&source);
      let input_g_size = get_gzipped_size(&non_shaken_code);
      let output_g_size = get_gzipped_size(&codegen_return.code);
      input_g_total += input_g_size;
      output_g_total += output_g_size;
      println!(
        "    [GZIP] Original: {}B\t{}: {}B\t{}: {}B\tRate: {:.2}%",
        source_g_size,
        if args.minify { "Shaken&Minified" } else { "Shaken" },
        output_g_size,
        if args.minify { "Minified" } else { "Copied" },
        input_g_size,
        (output_g_size as f64 / input_g_size as f64) * 100.0,
      );
    }

    let elapsed = start_time.elapsed();
    println!("-------------------");
    println!(
      "Completed in {:?}\tRate: {:.2}%\tGzipped Rate: {:.2}%",
      elapsed,
      (output_total as f64 / input_total as f64) * 100.0,
      (output_g_total as f64 / input_g_total as f64) * 100.0,
    );
  }
}

fn get_gzipped_size(content: &str) -> usize {
  let mut encoder = GzEncoder::new(Vec::new(), Compression::best());
  encoder.write_all(content.as_bytes()).unwrap();
  encoder.finish().unwrap().len()
}
