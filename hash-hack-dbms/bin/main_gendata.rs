use clap::{IntoApp, Parser, Subcommand};
use clap_complete::Shell;
use hash_hack_dbms::{
    gendata::{gen_collision_data_bili2, gen_data_bili2, gen_collision_resolve_data_bili2},
    shell::gen_completions,
};


/// Data Generator
#[derive(Parser)]
#[clap(name = "hhgd", bin_name = "hhgd")]
struct Cli {
    /// Genrerate completion for bin
    #[clap(long = "generate", arg_enum)]
    generator: Option<Shell>,

    #[clap(subcommand)]
    command: Option<SubCommand>,
}

#[derive(Subcommand)]
enum SubCommand {
    /// generate bilibili uid table by id
    Bili2 {
        #[clap(validator=format_u32_str)]
        id: u32,
    },

    /// generate dup db
    Dup {},

    #[clap(subcommand)]
    Resolve(Resolve)
}

#[derive(Subcommand)]
enum Resolve {
    Rehash
}


fn format_u32_str(s: &str) -> Result<u32, String> {
    let s = s.replace("_", "");
    u32::from_str_radix(&s, 10).or(Err(s))
}

fn main() {
    let cli = Cli::parse();

    if let Some(generator) = cli.generator {
        let mut cmd = Cli::command();
        gen_completions(generator, &mut cmd);
        return;
    }

    if let Some(command) = cli.command {
        match command {
            SubCommand::Bili2 { id } => gen_data_bili2(id),
            SubCommand::Dup {} => gen_collision_data_bili2(),
            SubCommand::Resolve(resolve) => match resolve {
                Resolve::Rehash => {
                    gen_collision_resolve_data_bili2()
                },
            }
        }
    }
}
