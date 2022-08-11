// SPDX-License-Identifier: Apache-2.0
use anchor_syn::idl::{Idl, IdlType, IdlTypeDefinitionTy};
use clap::ArgMatches;
use serde_json::Value as JsonValue;
use std::{
    ffi::{OsStr, OsString},
    fs::File,
    io::Write,
    path::PathBuf,
    process::exit,
};

pub fn idl(matches: &ArgMatches) {
    let files = matches.get_many::<OsString>("INPUT").unwrap();

    let base = matches.get_one::<OsString>("OUTPUT").map(PathBuf::from);

    for file in files {
        single_file(file, &base);
    }
}

fn single_file(file: &OsStr, base: &Option<PathBuf>) {
    let f = match File::open(file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{}: error: {}", file.to_string_lossy(), e);
            exit(1);
        }
    };

    let idl: Idl = match serde_json::from_reader(f) {
        Ok(idl) => idl,
        Err(e) => {
            eprintln!("{}: error: {}", file.to_string_lossy(), e);
            exit(1);
        }
    };

    let filename = format!("{}.sol", idl.name);

    let path = if let Some(base) = base {
        base.join(filename)
    } else {
        PathBuf::from(filename)
    };

    println!(
        "{}: info: creating '{}'",
        file.to_string_lossy(),
        path.display()
    );

    let f = match File::create(&path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("{}: error: {}", path.display(), e);
            exit(1);
        }
    };

    if let Err(e) = write_solidity(&idl, f) {
        eprintln!("{}: error: {}", path.display(), e);
        exit(1);
    }
}

fn write_solidity(idl: &Idl, mut f: File) -> Result<(), std::io::Error> {
    if let Some(program_id) = program_id(idl) {
        writeln!(
            f,
            "anchor_{} constant {} = anchor_{}(address'{}');\n",
            idl.name, idl.name, idl.name, program_id
        )?;
    }

    for ty_def in &idl.types {
        // print doc comment
        match &ty_def.ty {
            IdlTypeDefinitionTy::Enum { variants } => {
                if variants.iter().any(|variant| variant.fields.is_some()) {
                    eprintln!(
                        "enum {} has variants with fields, not supported in Solidity\n",
                        ty_def.name
                    );
                    continue;
                }
                write!(f, "enum {} {{", ty_def.name)?;
                let mut iter = variants.iter();
                let mut next = iter.next();
                while let Some(e) = next {
                    next = iter.next();

                    writeln!(f, "\t{}{}", e.name, if next.is_some() { "," } else { "" })?;
                }
                writeln!(f, "}}")?;
            }
            IdlTypeDefinitionTy::Struct { fields } => {
                let badtys: Vec<String> = fields
                    .iter()
                    .filter_map(|field| idltype_to_solidity(&field.ty).err())
                    .collect();

                if badtys.is_empty() {
                    writeln!(f, "struct {} {{", ty_def.name)?;
                    let mut iter = fields.iter();
                    let mut next = iter.next();
                    while let Some(e) = next {
                        next = iter.next();

                        writeln!(
                            f,
                            "\t{}\t{}{}",
                            e.name,
                            idltype_to_solidity(&e.ty).unwrap(),
                            if next.is_some() { "," } else { "" }
                        )?;
                    }
                    writeln!(f, "}}")?;
                } else {
                    eprintln!(
                        "struct {} has fields of type {} which is not supported on Solidity",
                        ty_def.name,
                        badtys.join(", ")
                    );
                }
            }
        }
    }

    if let Some(events) = &idl.events {
        for event in events {
            let badtys: Vec<String> = event
                .fields
                .iter()
                .filter_map(|field| idltype_to_solidity(&field.ty).err())
                .collect();

            if badtys.is_empty() {
                writeln!(f, "event {} {{", event.name)?;
                let mut iter = event.fields.iter();
                let mut next = iter.next();
                while let Some(e) = next {
                    next = iter.next();

                    writeln!(
                        f,
                        "\t{}\t{}{}{}",
                        idltype_to_solidity(&e.ty).unwrap(),
                        if e.index { " indexed " } else { " " },
                        e.name,
                        if next.is_some() { "," } else { "" }
                    )?;
                }
                writeln!(f, "}}")?;
            } else {
                eprintln!(
                    "event {} has fields of type {} which is not supported on Solidity",
                    event.name,
                    badtys.join(", ")
                );
            }
        }
    }

    writeln!(f, "interface anchor_{} {{", idl.name)?;

    for instr in &idl.instructions {
        let badtys: Vec<String> = instr
            .args
            .iter()
            .filter_map(|field| idltype_to_solidity(&field.ty).err())
            .collect();
        if badtys.is_empty() {
            write!(f, "\tfunction {}(", instr.name)?;

            let mut iter = instr.args.iter();
            let mut next = iter.next();
            while let Some(e) = next {
                next = iter.next();

                write!(
                    f,
                    "{} {}{}",
                    idltype_to_solidity(&e.ty).unwrap(),
                    e.name,
                    if next.is_some() { "," } else { "" }
                )?;
            }
            writeln!(f, ") external;")?;
        } else {
            eprintln!(
                "instructions {} has arguments of type {} which is not supported on Solidity",
                instr.name,
                badtys.join(", ")
            );
        }
    }

    writeln!(f, "}}")?;

    Ok(())
}

fn idltype_to_solidity(ty: &IdlType) -> Result<String, String> {
    match ty {
        IdlType::Bool => Ok("bool".to_string()),
        IdlType::U8 => Ok("uint8".to_string()),
        IdlType::I8 => Ok("int8".to_string()),
        IdlType::U16 => Ok("uint16".to_string()),
        IdlType::I16 => Ok("int16".to_string()),
        IdlType::U32 => Ok("uint32".to_string()),
        IdlType::I32 => Ok("int32".to_string()),
        IdlType::U64 => Ok("uint64".to_string()),
        IdlType::I64 => Ok("int64".to_string()),
        IdlType::U128 => Ok("uint128".to_string()),
        IdlType::I128 => Ok("int128".to_string()),
        IdlType::F32 => Err("f32".to_string()),
        IdlType::F64 => Err("f64".to_string()),
        IdlType::Bytes => Ok("bytes".to_string()),
        IdlType::String => Ok("string".to_string()),
        IdlType::PublicKey => Ok("address".to_string()),
        IdlType::Option(ty) => Err(format!(
            "Option({})",
            match idltype_to_solidity(ty) {
                Ok(ty) => ty,
                Err(ty) => ty,
            }
        )),
        IdlType::Defined(s) => {
            // TODO: what is it!
            Err(format!("Defined({})", s))
        }
        IdlType::Vec(ty) => match idltype_to_solidity(ty) {
            Ok(ty) => Ok(format!("{}[]", ty)),
            Err(ty) => Err(format!("{}[]", ty)),
        },
        IdlType::Array(ty, size) => match idltype_to_solidity(ty) {
            Ok(ty) => Ok(format!("{}[{}]", ty, size)),
            Err(ty) => Err(format!("{}[{}]", ty, size)),
        },
    }
}

fn program_id(idl: &Idl) -> Option<&String> {
    if let Some(JsonValue::Object(metadata)) = &idl.metadata {
        if let Some(JsonValue::String(address)) = metadata.get("address") {
            return Some(address);
        }
    }

    None
}
