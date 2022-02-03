use std::collections::HashMap;
use std::io::Write;
use std::{fs::File, io::Read};
use clap::StructOpt;
use clap_derive::Parser;

#[derive(Parser, Debug)]
struct Args {
    #[clap(short, long)]
    input: String,

    #[clap(short, long)]
    output: String,
}

fn main() {
    static EMPTY: String = String::new();

    let args = Args::parse();

    let xml = &mut String::new();
    File::open(args.input)
        .unwrap()
        .read_to_string(xml)
        .expect("Unable to load SVD");
    let svd = svd_parser::parse(xml).unwrap();

    let mut writer = File::create(args.output).unwrap();

    writeln!(
        writer,
        r#"<html><body>
        <style type="text/css">
        body {{
            font-family: "Segoe UI", Tahoma, Geneva, Verdana, sans-serif;
            margin: 16;
        }}

        h1 {{
            background: white;
            position: sticky;
            top: 0;
        }}

        table, td {{
            width: 100%;
            border-collapse: collapse;
        }}

        table td {{
            width: 1%; 
            border: 1px solid black; 
            text-align: center;
        }}

        table td.header {{
            writing-mode: vertical-lr; 
            transform: rotate(180deg) translate(0px, 8px);
            width: 1%; border: none;
            text-align: start;
        }}

        a {{
            font-family: "Segoe UI", Tahoma, Geneva, Verdana, sans-serif;
            margin: 16;
            color: black;
        }}
    </style>
    "#
    )
    .ok();

    writeln!(
        writer,
        "<h1>Peripheral List</h1>",
    )
    .ok();

    for (i,peripheral) in svd.peripherals.iter().enumerate() {
        writeln!(
            writer,
            "<p><a href=\"#p{}\">{}</a></p>",
            i, peripheral.name
        )
        .ok();
    }

    writeln!(
        writer,
        "</br></br>",
    )
    .ok();


    for (i,peripheral) in svd.peripherals.iter().enumerate() {
        writeln!(
            writer,
            "<h1 id=\"p{}\">{} (Base 0x{:8x})</h1>",
            i, peripheral.name, peripheral.base_address
        )
        .ok();
        writeln!(
            writer,
            "<p>{}</p>",
            peripheral.description.as_ref().unwrap_or(&EMPTY)
        )
        .ok();

        if peripheral.interrupt.len() > 0 {
            writeln!(writer, "<h2>Peripheral Interrupts</h2>").ok();
            for interrupt in peripheral.interrupt.iter() {
                let desc = match &interrupt.description {
                    Some(desc) => desc.clone(),
                    None => EMPTY.clone(),
                };
    
                writeln!(
                    writer,
                    r#"<p><b>{}</b> <i>{}</i> {}</p>"#,
                    interrupt.name, interrupt.value, desc
                )
                .ok();
            }
            writeln!(writer, "</br>").ok();
        }

        for register in peripheral.registers() {
            let sz = match register {
                svd_parser::svd::MaybeArray::Single(_) => EMPTY.clone(),
                svd_parser::svd::MaybeArray::Array(_, dim) => {
                    format!("{}", dim.dim)
                }
            };
            let name = register.name.replace("%s", &format!("<0..{}>", sz));

            writeln!(
                writer,
                "<h2>{} (Offset 0x{:04x} Absolut 0x{:8x})</h2>",
                name,
                register.address_offset,
                peripheral.base_address + register.address_offset as u64
            )
            .ok();
            writeln!(
                writer,
                "<p>{}</p>",
                register.description.as_ref().unwrap_or(&EMPTY)
            )
            .ok();

            // show bits in table
            writeln!(writer, r#"<table>"#).ok();

            // field names
            let flds: HashMap<_, _> = register
                .fields()
                .map(|f| (f.bit_offset() + f.bit_width() / 2, f))
                .collect();
            writeln!(writer, "<tr>").ok();
            for bit in (0..32).rev() {
                let text = if let Some(field) = flds.get(&bit) {
                    &field.name
                } else {
                    &EMPTY
                };
                writeln!(writer, r#"<td class="header">{}</td>"#, text).ok();
            }
            writeln!(writer, "</tr>").ok();

            // field bit ranges (from > to)
            let mut flds: Vec<(_, _)> = register
                .fields()
                .map(|f| (f.bit_offset() + f.bit_width() - 1, f.bit_offset()))
                .rev()
                .collect();
            let mut at = 0;
            for i in (0..flds.len()).rev() {
                let (from, to) = flds[i];
                if to > at {
                    flds.insert(i + 1, (at + (to - at) - 1, at));
                    at = from + 1;
                } else {
                    at = from + 1;
                }
            }
            if flds.len() > 0 {
                let (f, _) = flds[0];
                if f < 31 {
                    flds.insert(0, (31, f + 1));
                }
            } else {
                flds.push((31, 0));
            }

            writeln!(writer, "<tr>").ok();
            for (from, to) in flds {
                let desc = if from != to {
                    format!("{} - {}", from, to)
                } else {
                    format!("{}", from)
                };
                let span = from - to + 1;
                writeln!(writer, r#"<td colspan="{}">{}</td>"#, span, desc).ok();
            }
            writeln!(writer, "</tr>").ok();

            // bits
            writeln!(writer, "<tr>").ok();
            for bit in (0..32).rev() {
                writeln!(writer, r#"<td>{}</td>"#, bit).ok();
            }
            writeln!(writer, "</tr>").ok();

            writeln!(writer, "<table>").ok();

            // describe fields
            for field in register.fields() {
                let desc = match &field.description {
                    Some(desc) => desc.to_string(),
                    None => EMPTY.clone(),
                };
                let access = match &field.access {
                    Some(access) => match access {
                        svd_parser::svd::Access::ReadOnly => "R".to_string(),
                        svd_parser::svd::Access::ReadWrite => "RW".to_string(),
                        svd_parser::svd::Access::ReadWriteOnce => "RWO".to_string(),
                        svd_parser::svd::Access::WriteOnce => "WO".to_string(),
                        svd_parser::svd::Access::WriteOnly => "W".to_string(),
                    },
                    None => "-".to_string(),
                };
                writeln!(
                    writer,
                    r#"<p><b>{}</b> <i>{}</i> {}</p>"#,
                    field.name, access, desc
                )
                .ok();
            }
            writeln!(writer, "</br>").ok();
        }

        writeln!(writer, "</br></br></br>").ok();
    }
    writeln!(writer, "</body></html>").ok();
}
