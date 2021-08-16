use std::{env, error::Error, fs, path::Path};

struct Region {
    variant: String,
    id: String,
}

fn main() -> Result<(), Box<dyn Error>> {
    println!("cargo:rerun-if-changed=regions.txt");
    let regions = include_str!("regions.txt")
        .lines()
        .map(|id| Region {
            variant: id
                .split('-')
                .map(|word| {
                    let mut chars = word.chars();
                    if let Some(a) = chars.next() {
                        a.to_uppercase()
                            .chain(chars.as_str().to_lowercase().chars())
                            .collect()
                    } else {
                        String::new()
                    }
                })
                .collect::<Vec<_>>()
                .join(""),
            id: id.into(),
        })
        .collect::<Vec<_>>();

    let dest_path = Path::new(&env::var("OUT_DIR")?).join("region.rs");

    // the enum
    let mut buf =
        "/// A list of AWS Regions supported by DynamoDB\n#[non_exhaustive]\npub enum Region {\n"
            .to_string();
    for region in &regions {
        buf.push_str("  ");
        buf.push_str(&region.variant);
        buf.push_str(",\n");
    }
    buf.push_str("}\n");

    // the impl
    buf.push_str("\nimpl Region {\n");

    buf.push_str("  /// Short region identifier\n");
    buf.push_str("  pub fn id(&self) -> &str {\n");
    buf.push_str("    match self {\n");
    for region in &regions {
        buf.push_str("      Region::");
        buf.push_str(&region.variant);
        buf.push_str(" => \"");
        buf.push_str(&region.id);
        buf.push_str("\",\n");
    }
    buf.push_str("    }\n  }\n");

    buf.push_str("  /// region specific dynamodb endpoint\n");
    buf.push_str("  pub fn endpoint(&self) -> &str {\n");
    buf.push_str("    match self {\n");
    for region in &regions {
        buf.push_str("      Region::");
        buf.push_str(&region.variant);
        buf.push_str(" => \"dynamodb.");
        buf.push_str(&region.id);
        buf.push_str(".amazonaws.com\",\n");
    }
    buf.push_str("    }\n  }\n");
    buf.push_str("}\n");

    // from str
    buf.push_str("\nimpl std::str::FromStr for Region {\n");
    buf.push_str("  type Err = String;\n");

    buf.push_str("  fn from_str(s: &str) ->  Result<Self, Self::Err> {\n");
    buf.push_str("    match s {\n");
    for region in &regions {
        buf.push_str("      \"");
        buf.push_str(&region.id);
        buf.push_str("\" => Ok(Region::");
        buf.push_str(&region.variant);
        buf.push_str("),\n");
    }
    buf.push_str("      _ => Err(\"invalid region\".into()),\n");
    buf.push_str("    }\n  }\n");
    buf.push_str("}\n");

    fs::write(&dest_path, buf)?;
    Ok(())
}
