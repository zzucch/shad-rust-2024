use std::{
    collections::HashSet,
    env::args,
    fs::File,
    io::{stdout, BufRead, BufReader, BufWriter, Result, Write},
};

fn main() -> Result<()> {
    let args = args().collect::<Vec<String>>();
    if args.len() < 3 {
        eprintln!("usage: {} [file_1] [file_2]", args[0]);
        return Ok(());
    }

    let mut first_file_lines = read_file(&args[1])?;

    let second_file = File::open(&args[2])?;
    let reader = BufReader::new(second_file);
    let mut writer = BufWriter::new(stdout());

    for line in reader.lines() {
        let line = line?;

        if first_file_lines.contains(&line) {
            writer.write_all(line.as_bytes())?;
            writer.write_all(b"\n")?;

            first_file_lines.take(&line);
        }
    }

    writer.flush()?;

    Ok(())
}

fn read_file(path: &String) -> Result<HashSet<String>> {
    let mut lines = HashSet::new();

    let file = File::open(path)?;
    let reader = BufReader::new(file);

    for line in reader.lines() {
        lines.insert(line?);
    }

    Ok(lines)
}
