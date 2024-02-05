#![feature(async_fn_in_trait)]
pub mod data_type;
pub mod messages;
pub mod model;
pub mod polars_df;
pub mod test_data;
pub mod utils;

#[cfg(test)]
pub(crate) mod tests {
    use log::LevelFilter;
    use simplelog::{ColorChoice, CombinedLogger, Config, TermLogger, TerminalMode};

    pub(crate) fn init_log() {
        CombinedLogger::init(vec![TermLogger::new(
            LevelFilter::Trace,
            Config::default(),
            TerminalMode::Mixed,
            ColorChoice::Auto,
        )])
        .unwrap_or_else(|_| {});
    }

    // #[ctor::ctor]
    // fn init() {
    //     init_log();
    // }

    #[test]
    fn test_1() {
        //std::env::set_var("POLARS_FMT_MAX_ROWS", "20");

        //parquet_scan();
        println!("Ok 2")
    }
    //
    //
    //
    // #[test]
    // fn test_simp_1() {
    //     std::env::set_var("POLARS_FMT_MAX_ROWS", "20");
    //
    //     simp_1();
    // }
}
