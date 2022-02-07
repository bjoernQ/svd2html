use std::{
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
};

use anyhow::Result;
use clap::Parser;
use lazy_static::lazy_static;
use minijinja::{context, value::Value, Environment, Source, State};
use svd_parser::svd::{Access, FieldInfo, MaybeArray, PeripheralInfo, RegisterInfo};

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

fn render_index(chip: &str, peripherals: &[&PeripheralInfo]) -> Result<String> {
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

fn render_peripheral(chip: &str, peripheral: &PeripheralInfo) -> Result<String> {
    // Build the template context.
    let ctx = context! {
        chip        => chip,
        peripheral  => peripheral.name.clone(),
        address     => format!("0x{:08x}", peripheral.base_address),
        description => peripheral.description.clone().unwrap_or_default(),
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
                description => i.description.clone().unwrap_or_default()
            }
        })
        .collect::<Vec<_>>()
}

fn registers(peripheral: &PeripheralInfo) -> Vec<Value> {
    peripheral
        .registers()
        .map(|register| {
            let (ri, dim) = match register {
                MaybeArray::Single(ri) => (ri, 0u32),
                MaybeArray::Array(ri, de) => (ri, de.dim),
            };

            let absolute = peripheral.base_address + ri.address_offset as u64;

            context! {
                name        => ri.name.replace("%s", &format!("<0..{dim}>")),
                description => ri.description.clone().unwrap_or_default(),
                offset      => format!("0x{:04x}", ri.address_offset),
                absolute    => format!("0x{:08x}", absolute),
                fields      => fields(register),
            }
        })
        .collect::<Vec<_>>()
}

fn fields(register: &MaybeArray<RegisterInfo>) -> Vec<Value> {
    fields_with_spans(register)
        .iter()
        .map(|(f, from, to)| {
            let (name, desc, access) = field_info(f);

            context! {
                name        => name,
                description => desc,
                access      => access,

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

fn fields_with_spans(
    register: &MaybeArray<RegisterInfo>,
) -> Vec<(Option<&MaybeArray<FieldInfo>>, u32, u32)> {
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

    if !fields.is_empty() {
        let (f, from, _) = fields[0];
        if from < 31 {
            fields.insert(0, (f, 31, from + 1));
        }
    } else {
        fields.push((None, 31, 0));
    }

    fields
}

fn field_info(field: &Option<&MaybeArray<FieldInfo>>) -> (String, String, String) {
    let mut name = String::new();
    let mut desc = String::new();
    let mut access = String::from("-");

    if let Some(f) = field {
        name = f.name.clone();

        if let Some(description) = &f.description {
            desc = description.to_owned();
        }

        access = match &f.access {
            Some(access) => match access {
                Access::ReadOnly => "R",
                Access::ReadWrite => "RW",
                Access::ReadWriteOnce => "RWO",
                Access::WriteOnce => "WO",
                Access::WriteOnly => "W",
            },
            None => "-",
        }
        .to_string();
    }

    (name, desc, access)
}

fn write_html(source: &str, path: &Path) -> Result<()> {
    eprintln!("Writing: {}", path.display());

    let mut file = File::create(path)?;
    file.write_all(source.as_bytes())?;

    Ok(())
}
