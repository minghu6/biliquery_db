use clap::{IntoApp, Parser, Subcommand};
use clap_complete::Shell;
use hash_hack_dbms::{
    query::{print_dbmeta, query_bili2, query_collision_rehash_resolve},
    shell::gen_completions,
};


/// Query Client
#[derive(Parser)]
#[clap(name = "hhq", bin_name = "hhq")]
struct Cli {
    /// Genrerate completion for bin
    #[clap(long = "generate", arg_enum)]
    generator: Option<Shell>,

    #[clap(subcommand)]
    command: Option<SubCommand>,
}

#[derive(Subcommand)]
enum SubCommand {
    /// bilibili query
    Bili2 { id: String },

    /// check database meta
    Config {},
}

fn format_hex_str(s: &str) -> Result<u32, String> {
    u32::from_str_radix(&s, 16).or(Err(s.to_string()))
}



fn main() {
    let cli = Cli::parse();

    if let Some(generator) = cli.generator {
        let mut cmd = Cli::command();
        gen_completions(generator, &mut cmd);
    }

    if let Some(command) = cli.command {
        match command {
            SubCommand::Bili2 { id } => {
                // let ids: Vec<u32> = ids
                // .into_iter()
                // .map(|id| format_hex_str(&id).unwrap())
                // .collect();
                let id = format_hex_str(&id).unwrap();
                let res = query_bili2(id).unwrap();

                if res.is_empty() {
                    // query collision resolve
                    println!("Not Found in Normal.");
                } else {
                    println!("Normal:");
                    println!("{:#?}", res);
                }

                let resolve_res = query_collision_rehash_resolve(id).unwrap();
                if resolve_res.is_empty() {
                    println!("Resolve Failed.")
                } else {
                    println!("Resolved: ");
                    println!("{:#?}", res)
                }
            }
            SubCommand::Config {} => print_dbmeta(),
        }
    }
}
