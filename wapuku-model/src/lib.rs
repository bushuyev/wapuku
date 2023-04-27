pub(crate) mod polars_df;
pub(crate) mod model;
mod data_type;


#[cfg(test)]
pub(crate) mod tests {
    use log::LevelFilter;
    use polars::prelude::*;
    use polars::df;
    use crate::model::*;
    use simplelog::{ColorChoice, CombinedLogger, Config, TerminalMode, TermLogger};
    use crate::polars_df::*;

    pub(crate) fn init_log() {
        CombinedLogger::init(
            vec![
                TermLogger::new(LevelFilter::Trace, Config::default(), TerminalMode::Mixed, ColorChoice::Auto),
            ]
        ).unwrap_or_else(|_|{});
    }

    #[ctor::ctor]
    fn init() {
        init_log();
    }

    // #[test]
    // fn test_1() {
    //     std::env::set_var("POLARS_FMT_MAX_ROWS", "20");
    // 
    //     parquet_scan();
    // 
    // }
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


