use std::{
    fs::{self, File},
    io::Write,
    path::PathBuf,
};

use anyhow::Result;
use clap::Parser;
use lazy_static::lazy_static;
use minijinja::{context, value::Value, Environment, Source, State};
use svd_parser::svd::{Access, MaybeArray, PeripheralInfo, RegisterInfo};

lazy_static! {
    static ref ENV: Environment<'static> = create_environment();
}

#[derive(Parser, Debug)]
#[clap(version)]
struct Opts {
    /// SVD file to parse
    #[clap(short, long)]
    input: PathBuf,

    /// Directory to write generated HTML files
    #[clap(short, long, default_value = "output")]
    output: PathBuf,
}

fn main() -> Result<()> {
    let opts = Opts::parse();

    // Create the output directory if it does not already exist.
    if !opts.output.exists() {
        fs::create_dir_all(&opts.output)?;
    }

    // Parse the 'input' file, which should be an SVD (aka XML).
    let xml = fs::read_to_string(&opts.input)?;
    let svd = svd_parser::parse(&xml).unwrap();

    // Convert the Vector of `MaybeArray<PeirpheralInfo>` to a Vector of just
    // `PeripheralInfo`.
    let peripherals = svd
        .peripherals
        .iter()
        .filter_map(|p| match p {
            MaybeArray::Single(pi) => Some(pi),
            MaybeArray::Array(..) => unreachable!(), // Is it, though? ;)
        })
        .collect::<Vec<_>>();

    // Render each peripheral page. List each interupt and register, as well as each
    // register's fields.
    let chip = svd.name.clone();
    for peripheral in &peripherals {
        let filename = format!("{}.html", peripheral.name);
        let html = render_peripheral(&chip, peripheral)?;
        write_html(&html, &opts.output.join(filename))?;
    }

    // Render the index page, which lists all peripherals for a device with links to
    // each peripheral's page.
    let html = render_index(&chip, &peripherals)?;
    write_html(&html, &opts.output.join("index.html"))?;

    Ok(())
}

fn create_environment() -> Environment<'static> {
    use minijinja::{Error, ErrorKind};

    let mut env = Environment::new();
    let mut src = Source::new();

    src.load_from_path("templates", &["html"]).unwrap();
    env.set_source(src);

    // Define a custom function which allows us to include static files within our
    // templates.
    fn include_file(_state: &State, name: String) -> std::result::Result<String, Error> {
        fs::read_to_string(&name).map_err(|e| {
            Error::new(ErrorKind::ImpossibleOperation, "cannot load file").with_source(e)
        })
    }
    env.add_function("include_file", include_file);

    env
}

fn render_index(chip: &String, peripherals: &Vec<&PeripheralInfo>) -> Result<String> {
    // Iterate through all peripherals, and constructor a Vector of Context
    // containing the name and description for each.
    let peripheral_info = peripherals
        .iter()
        .map(|p| {
            context! {
                name        => p.name.clone(),
                description => p.description.clone().unwrap_or_default(),
            }
        })
        .collect::<Vec<_>>();

    // Build the template context.
    let ctx = context! {
        chip        => chip,
        peripherals => peripheral_info,
    };

    // Render the template to HTML using the context defined above.
    let tmpl = ENV.get_template("index.html")?;
    let html = tmpl.render(ctx)?;

    Ok(html)
}

fn render_peripheral(chip: &String, peripheral: &PeripheralInfo) -> Result<String> {
    // Build the template context.
    let ctx = context! {
        chip        => chip,
        peripheral  => peripheral.name.clone(),
        description => if let Some(desc) = &peripheral.description {
            desc.to_owned()
        } else {
            String::new()
        },
        interrupts  => interrupts(peripheral),
        registers   => registers(peripheral),
    };

    // Render the template to HTML using the context defined above.
    let tmpl = ENV.get_template("peripheral.html")?;
    let html = tmpl.render(ctx)?;

    Ok(html)
}

fn interrupts(peripheral: &PeripheralInfo) -> Vec<Value> {
    peripheral
        .interrupt
        .iter()
        .map(|i| {
            context! {
                name        => i.name.clone(),
                value       => i.value.to_string(),
                description => if let Some(desc) = &i.description {
                    desc.to_owned()
                } else {
                    String::new()
                }
            }
        })
        .collect::<Vec<_>>()
}

fn registers(peripheral: &PeripheralInfo) -> Vec<Value> {
    peripheral
        .registers()
        .map(|register| {
            let (ri, size) = match register {
                MaybeArray::Single(ri) => (ri, 0u32),
                MaybeArray::Array(ri, de) => (ri, de.dim),
            };

            let absolute = peripheral.base_address + ri.address_offset as u64;

            context! {
                name        => ri.name.replace("%s", &format!("<0..{size}>")),
                description => if let Some(desc) = &ri.description {
                    desc.to_owned()
                } else {
                    String::new()
                },
                offset      => format!("0x{:04x}", ri.address_offset),
                absolute    => format!("0x{:08x}", absolute),
                fields      => context! {
                    spans        => fields(register),
                    descriptions => field_descriptions(register),
                },
            }
        })
        .collect::<Vec<_>>()
}

fn fields(register: &MaybeArray<RegisterInfo>) -> Vec<Value> {
    let mut fields = register
        .fields()
        .map(|f| {
            let from = f.bit_offset() + f.bit_width() - 1;
            let to = f.bit_offset();

            (Some(f), from, to)
        })
        .rev()
        .collect::<Vec<(_, _, _)>>();

    let mut at = 0;
    for i in (0..fields.len()).rev() {
        let (f, from, to) = fields[i];

        if to > at {
            fields.insert(i + 1, (f, at + (to - at) - 1, at));
        }

        at = from + 1;
    }

    if fields.len() > 0 {
        let (f, from, _) = fields[0];
        if from < 31 {
            fields.insert(0, (f, 31, from + 1));
        }
    } else {
        fields.push((None, 31, 0));
    }

    fields
        .iter()
        .map(|(f, from, to)| {
            context! {
                name => if let Some(field) = f {
                    field.name.to_owned()
                } else {
                    String::new() // TODO: should this be RESERVED in any case?
                },
                span => from - to + 1,
                text => if from == to {
                    format!("{}", from)
                } else {
                    format!("{} - {}", from, to)
                },
            }
        })
        .collect::<Vec<_>>()
}

fn field_descriptions(register: &MaybeArray<RegisterInfo>) -> Vec<Value> {
    register
        .fields()
        .map(|f| {
            let desc = match &f.description {
                Some(d) => d.to_owned(),
                None => String::new(),
            };

            let access = match &f.access {
                Some(access) => match access {
                    Access::ReadOnly => "R",
                    Access::ReadWrite => "RW",
                    Access::ReadWriteOnce => "RWO",
                    Access::WriteOnce => "WO",
                    Access::WriteOnly => "W",
                },
                None => "-",
            };

            context! {
                name        => f.name.clone(),
                description => desc,
                access      => access,
            }
        })
        .collect::<Vec<_>>()
}

fn write_html(source: &String, path: &PathBuf) -> Result<()> {
    eprintln!("Writing: {}", path.display());

    let mut file = File::create(path)?;
    file.write_all(source.as_bytes())?;

    Ok(())
}
