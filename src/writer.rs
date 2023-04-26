/// Wraps the `stdout.lock()` in a `csv::Writer` and writes the rows.
/// The `csv::Writer` is already buffered so there is no need to wrap
/// `stdout.lock()` in a `io::BufWriter`.
pub(crate) async fn print_csv_rows<S: serde::Serialize>(rows: &Vec<S>) -> std::io::Result<()>{
    let stdout = std::io::stdout();
    let lock = stdout.lock();
    let mut wtr =
        csv::WriterBuilder::new()
            .has_headers(true)
            .delimiter(b';')
            .from_writer(lock);

    let mut err = None;
    rows.iter().for_each(|row|
        wtr.serialize(row)
            .unwrap_or_else(|e| {
                err = Some(e);
                Default::default()
            })
    );
    err.map_or(Ok(()), Err)?;
    Ok(())
}