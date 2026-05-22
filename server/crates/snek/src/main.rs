mod expr;
mod instrs;
mod parser;
mod repl;
mod stack;
use clap::Parser;
use std::fs::File;
use std::io::prelude::*;

use crate::instrs::Instr;

#[derive(clap::Parser, Debug)]
#[command(version, about, long_about = None)]
struct CLI {
    /// Start the interactive REPL
    #[arg(long)]
    repl: bool,

    /// Source .snek file to compile
    input: Option<String>,

    /// Output assembly file
    output: Option<String>,
}

fn main() -> std::io::Result<()> {
    let cli = CLI::parse();

    if cli.repl {
        repl::repl();
        return Ok(());
    }

    let in_name = cli
        .input
        .expect("expected an input .snek file unless --repl is set");
    let out_name = cli
        .output
        .expect("expected an output assembly file unless --repl is set");

    let mut in_file = File::open(in_name)?;
    let mut in_contents = String::new();
    in_file.read_to_string(&mut in_contents)?;

    let prog = parser::parse(&in_contents);
    let mut ctx = expr::Ctxt::new();

    let exit_lbl = "program_exit".to_string();

    let asm_program = ctx.compile_prog(&prog, &exit_lbl);

    let result = asm_program
        .iter()
        .map(Instr::to_string)
        .collect::<Vec<_>>()
        .join("\n");

    // code generation
    let asm_program = format!(
        "section .text\n\
        global our_code_starts_here\n\
        extern snek_error\n\
        extern snek_print\n\
        {}\n\
        ",
        result
    );

    let mut out_file = File::create(out_name)?;
    out_file.write_all(asm_program.as_bytes())?;

    Ok(())
}
