use std::collections::{HashMap, HashSet};
use std::io::{Cursor, Read};
use std::iter::once;

use ::zip::result::*;
use ::zip::*;
use itertools::Either;
use log::{debug, warn};
use polars::datatypes::DataType::{Categorical, Datetime};
use polars::datatypes::TimeUnit::Milliseconds;
use polars::export::arrow::io::iterator::StreamingIterator;
use polars::export::chrono;
use polars::export::chrono::{NaiveDate, NaiveDateTime, NaiveTime, ParseResult};
use polars::frame::UniqueKeepStrategy::Any;
use polars::io::parquet::*;
use polars::prelude::AnyValue::Null;
use polars::prelude::FunctionExpr::Exp;
use polars::prelude::StartBy::WindowBound;
use polars::prelude::*;
use polars::series::IsSorted;

use polars::time::Duration;

use crate::data_type::WapukuDataType;
use crate::model::*;
use crate::utils::*;

pub fn format_date_str<E: AsRef<[Expr]>>(format: &str, args: E) -> PolarsResult<Expr> {
    let mut args: std::collections::VecDeque<Expr> = args.as_ref().to_vec().into();

    // Parse the format string, and separate substrings between placeholders
    let segments: Vec<&str> = format.split("{}").collect();

    polars_ensure!(
        segments.len() - 1 == args.len(),
        ShapeMismatch: "number of placeholders should equal the number of arguments"
    );

    let mut exprs: Vec<Expr> = Vec::new();

    for (i, s) in segments.iter().enumerate() {
        if i > 0 {
            if let Some(arg) = args.pop_front() {
                exprs.push(arg);
            }
        }

        if !s.is_empty() {
            exprs.push(lit(s.to_string()))
            // exprs.push(lit("zzz"))
        }
    }

    Ok(concat_str(exprs, ""))
}

impl From<PolarsError> for WapukuError {
    fn from(value: PolarsError) -> Self {
        WapukuError::DataFrame {
            msg: value.to_string(),
        }
    }
}

impl From<&PolarsError> for WapukuError {
    fn from(value: &PolarsError) -> Self {
        WapukuError::DataFrame {
            msg: value.to_string(),
        }
    }
}

const NA: &str = "n/a";

#[derive(Debug)]
pub struct PolarsData {
    df: DataFrame,
    property_sets: Vec<SimplePropertiesSet>,
    name: String,
}

impl From<ZipError> for WapukuError {
    fn from(value: ZipError) -> Self {
        WapukuError::DataLoad {
            msg: value.to_string(),
        }
    }
}

impl From<std::io::Error> for WapukuError {
    fn from(value: std::io::Error) -> Self {
        WapukuError::DataLoad {
            msg: value.to_string(),
        }
    }
}

impl PolarsData {
    pub fn new(df: DataFrame, name: String) -> Self {
        //TODO move to add df
        // parquet_scan();

        let properties = df
            .schema()
            .iter_fields()
            .map(|f| DataProperty::new(WapukuDataType::Numeric, f.name))
            .collect();
        Self {
            df,
            property_sets: vec![SimplePropertiesSet::new(properties, "item_1")],
            name,
        }
    }

    fn group_by_categoric(&self, frame_id: u128, column: String) -> Result<Histogram, WapukuError> {
        debug!("group_by_categoric column={:?}", column);

        let groupby_df = self
            .df
            .clone()
            .lazy()
            .group_by([col(column.as_str())])
            .agg([count().alias("count")])
            .sort("count", Default::default())
            .collect()?;

        debug!("groupby_df={:?}", groupby_df);

        if groupby_df.shape().0 > 1000 {
            return Err(WapukuError::DataLoad {
                msg: "Tooo big".into(),
            }); //TODO
        }

        let mut val_count = groupby_df.iter();

        Ok(Histogram::new(
            frame_id,
            column,
            std::iter::zip(
                val_count.next().expect("val").iter(),
                val_count.next().expect("count").iter(),
            )
            .fold(Vec::new(), |mut vec, vv| {
                if let (AnyValue::String(v1), AnyValue::UInt32(v2)) = vv {
                    vec.push((v1.to_string(), v2));
                } else {
                    warn!("unexpected values in build_histogram: {:?}", vv);
                }
                vec
            }),
        ))
    }

    fn group_by_datetime(&self, frame_id: u128, column: String) -> Result<Histogram, WapukuError> {
        debug!("group_by_datetime column={:?}", column);

        let df = self
            .df
            .clone()
            .lazy()
            .filter(col(column.as_str()).is_not_null())
            .collect()?;
        let column_series = df.column(column.as_str())?.sort(false);

        // let mut min = NaiveDateTime::MIN.timestamp_millis();
        // let mut max = NaiveDateTime::MAX.timestamp_millis();

        debug!("s.dtype()={:?}", column_series.dtype());

        let (mut min_value, mut max_value) = (
            column_series.min()?.unwrap_or(0),
            column_series.max()?.unwrap_or(0),
        );

        let mut time_unit = TimeUnit::Milliseconds;

        match column_series.dtype() {
            DataType::Date => {}
            DataType::Datetime(unit, _) => {
                match unit {
                    TimeUnit::Nanoseconds => {
                        time_unit = TimeUnit::Nanoseconds;
                        // min_value /= 1_000_000;
                        // max_value /= 1_000_000;
                        debug!(
                            "nanos min_value={:?} max_value={:?} min_dt={:?}, max_dt={:?}",
                            min_value,
                            max_value,
                            chrono::NaiveDateTime::from_timestamp_millis(min_value / 1_000_000),
                            chrono::NaiveDateTime::from_timestamp_millis(max_value / 1_000_000)
                        );
                    }
                    TimeUnit::Microseconds => {
                        time_unit = TimeUnit::Microseconds;
                        // min_value /= 1_000;
                        // max_value /= 1_000;
                        debug!(
                            "micros min_value={:?} max_value={:?} min_dt={:?}, max_dt={:?}",
                            min_value,
                            max_value,
                            chrono::NaiveDateTime::from_timestamp_micros(min_value),
                            chrono::NaiveDateTime::from_timestamp_millis(max_value)
                        );
                    }
                    Milliseconds => {
                        debug!(
                            "millis: min_value={:?} max_value={:?} min_dt={:?}, max_dt={:?}",
                            min_value,
                            max_value,
                            chrono::NaiveDateTime::from_timestamp_millis(min_value),
                            chrono::NaiveDateTime::from_timestamp_millis(max_value)
                        );
                    }
                }
            }
            DataType::Duration(_) => {}
            DataType::Time => {}
            _ => {
                warn!("Unexpected datatype"); //TODO
            }
        }

        // let start = s.min::<f64>().unwrap().floor() - 1.0;
        // let stop = s.max::<f64>().unwrap().ceil() + 1.0;

        let bin_count = 10;

        let interval = (max_value - min_value) / bin_count;
        let breaks: Vec<i64> = (0..(bin_count)).map(|b| min_value + b * interval).collect();

        let breakpoint_str = &"break_point";
        let bins = Series::new(breakpoint_str, breaks);

        let mut bins = bins.extend_constant(AnyValue::Datetime(max_value, time_unit, &None), 1)?;
        bins.set_sorted_flag(IsSorted::Ascending);

        let cuts_df = df![
            breakpoint_str => bins
        ]?;

        let category_str = "category";

        let cuts_df = cuts_df
            .lazy()
            .with_column(
                format_date_str(
                    "({}, {}]",
                    [
                        col(breakpoint_str)
                            .shift_and_fill(1, min_value)
                            .cast(column_series.dtype().to_owned()),
                        col(breakpoint_str).cast(column_series.dtype().to_owned()),
                    ],
                )?
                .alias(category_str),
            )
            .collect()?;

        debug!("cuts_df={:?}", cuts_df);

        let cuts = cuts_df
            .lazy()
            .with_columns([
                col(category_str).cast(DataType::Categorical(None, CategoricalOrdering::Lexical)),
                col(breakpoint_str)
                    .cast(column_series.dtype().to_owned())
                    // .cast(Datetime(Milliseconds, None))
                    .set_sorted_flag(IsSorted::Ascending),
            ])
            .collect()?;

        let out = column_series.clone().into_frame().join_asof(
            &cuts,
            column_series.name(),
            breakpoint_str,
            AsofStrategy::Forward,
            None,
            None,
        )?;

        let out = out
            .select(["category", column_series.name()])?
            .group_by(["category"])?
            .count()?;

        debug!("out={:?}", out);

        let groupby_df = cuts
            .left_join(&out, [category_str], [category_str])?
            .fill_null(FillNullStrategy::Zero)?
            .sort(["category"], false, false)?;

        debug!("groupby_df={:?}", groupby_df);

        let mut val_count = groupby_df.iter();
        val_count.next(); //skip break_point

        Ok(Histogram::new(
            frame_id,
            column,
            std::iter::zip(
                val_count.next().expect("cat").iter(),
                val_count.next().expect("property_2_count").iter(),
            )
            .fold(Vec::new(), |mut vec, vv| {
                if let (
                    AnyValue::Categorical(a, RevMapping::Local(b, _), _c),
                    AnyValue::UInt32(count),
                ) = vv
                {
                    // warn!("a={:?}, count={:?}", b.value(a as usize), count);
                    vec.push((
                        FloatReformatter::exec(b.value(a as usize)).to_string(),
                        count,
                    ));
                } else {
                    warn!("unexpected values in build_histogram: {:?}", vv);
                }
                vec
            }),
        ))
    }

    fn group_by_numeric(
        &self,
        frame_id: u128,
        column: String,
        bins: Option<usize>,
    ) -> Result<Histogram, WapukuError> {
        debug!("group_by_numeric column={:?}", column);

        let frame = self
            .df
            .clone()
            .lazy()
            .select([col(column.as_str()).hist(None, bins.or(Some(10)), true, false)]) //TODO include_breakpoint false
            .collect()?;
        let groupby_df = frame.column(column.as_str())?;

        // ┌─────────────┬───────────────────────────────────┬──────────────────┐
        // │ break_point ┆ category                          ┆ property_2_count │
        // │ ---         ┆ ---                               ┆ ---              │
        // │ f64         ┆ cat                               ┆ u32              │
        // ╞═════════════╪═══════════════════════════════════╪══════════════════╡

        Ok(Histogram::new(
            frame_id,
            column,
            groupby_df
                .struct_()?
                .into_iter()
                .fold(Vec::new(), |mut vec, any_x| {
                    vec.push((
                        FloatReformatter::exec(any_x[0].get_str().unwrap()).to_string(),
                        if let AnyValue::UInt32(v) = any_x[1] {
                            v
                        } else {
                            0
                        },
                    ));

                    vec
                }),
        ))
    }
}

impl Data for PolarsData {
    fn load(data: Box<Vec<u8>>, name: Box<String>) -> Result<Vec<Self>, WapukuError> {
        if name.ends_with("csv") {
            load_csv(data).map(|d| vec![PolarsData::new(d, *name)])
        } else if name.ends_with("parquet") {
            load_parquet(data).map(|d| vec![PolarsData::new(d, *name)])
        } else if name.ends_with("zip") {
            load_zip(data).map(|d_vec| {
                d_vec
                    .into_iter()
                    .map(|(df, entry_name)| PolarsData::new(df, format!("{}/{}", name, entry_name)))
                    .collect()
            })
        } else {
            Err(WapukuError::General {
                msg: String::from("I can load only csv or parquet files"),
            })
        }
    }

    fn name(&self) -> String {
        self.name.clone()
    }

    fn all_sets(&self) -> Vec<&dyn PropertiesSet> {
        self.property_sets.iter().fold(vec![], |mut props, p| {
            props.push(p);

            props
        })
    }

    fn all_properties(&self) -> HashSet<&dyn Property> {
        self.property_sets
            .iter()
            .flat_map(|property_set| property_set.properties().into_iter())
            .collect()
    }

    fn build_grid(
        &self,
        property_x: PropertyRange,
        property_y: PropertyRange,
        x_n: u8,
        y_n: u8,
        group_volume_property: &str,
    ) -> GroupsGrid {
        let property_x_name = property_x.property().name().as_str();
        let property_y_name = property_y.property().name().as_str();

        let properties_df = self.df.select([property_x_name, property_y_name]).unwrap();

        let min_df = properties_df.min_horizontal().unwrap().unwrap();
        let max_df = properties_df.max_horizontal().unwrap().unwrap();
        // debug!("min_df={:?}, max_df={:?}", min_df, max_df);

        let min_x = property_x.min().unwrap_or(
            min_df
                .get(0)
                .expect("0")
                .try_extract::<f32>()
                .expect("min_x") as i64,
        ) as f32;
        let max_x = property_x.max().unwrap_or(
            max_df
                .get(0)
                .expect("0")
                .try_extract::<f32>()
                .expect("max_x") as i64,
        ) as f32;

        let min_y = property_y.min().unwrap_or(
            min_df
                .get(0)
                .expect("0")
                .try_extract::<f32>()
                .expect("min_y") as i64,
        ) as f32;
        let max_y = property_y.max().unwrap_or(
            max_df
                .get(0)
                .expect("0")
                .try_extract::<f32>()
                .expect("max_y") as i64,
        ) as f32;

        let property_x_step = (((max_x - min_x) / x_n as f32).ceil()) as i64;

        let property_y_step = (((max_y - min_y) / y_n as f32).ceil()) as i64;

        // debug!("wapuku: min_df={:?} max_df={:?}", min_df, max_df);
        debug!(
            "wapuku: min/max_x={:?}, min/max_y={:?} property_x_step={:?}, property_y_step={:?}",
            (min_x, max_x),
            (min_y, max_y),
            property_x_step,
            property_y_step
        );

        let df = if max_x == min_x {
            group_by_1(
                &self.df,
                property_y_name,
                property_y_step,
                [col(group_volume_property).count().alias("volume")],
                min_y as i64 - property_y_step,
            )
            .unwrap()
        } else if max_y == min_y {
            group_by_1(
                &self.df,
                property_x_name,
                property_x_step,
                [col(group_volume_property).count().alias("volume")],
                min_x as i64 - property_x_step,
            )
            .unwrap()
        } else {
            group_by_2(
                &self.df,
                property_x_name,
                property_x_step,
                property_y_name,
                property_y_step,
                [col(group_volume_property).count().alias("volume")],
                min_x as i64 - property_x_step,
                min_y as i64 - property_y_step,
            )
            .unwrap()
        };

        debug!("1. wapuku: df={:?}", df);

        // // https://stackoverflow.com/questions/72440403/iterate-over-rows-polars-rust df is small here, should be ok
        // df.as_single_chunk_par();
        // let mut iters = df.columns(["primary_field_group", "secondary_group_field", "volume"]).expect("grouping columns")
        //     .iter().map(|s| s.iter()).collect::<Vec<_>>();
        //

        let mut data_vec: Vec<Vec<Option<Box<dyn DataGroup>>>> = (0..y_n)
            .map(|_y| (0..x_n).map(|_x| None).collect())
            .collect();

        (0..y_n as usize).for_each(|y| {
            (0..x_n as usize).for_each(|x| {
                let x_0 = min_x as i64 + x as i64 * property_x_step;
                let x_1 = min_x as i64 + x as i64 * property_x_step + property_x_step;
                let y_0 = min_y as i64 + y as i64 * property_y_step;
                let y_1 = min_y as i64 + y as i64 * property_y_step + property_y_step;

                let count = df
                    .clone()
                    .lazy()
                    .filter(if max_x == min_x {
                        col("group_by_field")
                            .gt_eq(y_0)
                            .and(col("group_by_field").lt(y_1))
                    } else if max_y == min_y {
                        col("group_by_field")
                            .gt_eq(x_0)
                            .and(col("group_by_field").lt(x_1))
                    } else {
                        col("primary_field_group").gt_eq(x_0).and(
                            col("primary_field_group").lt(x_1).and(
                                col("secondary_group_field")
                                    .gt_eq(y_0)
                                    .and(col("secondary_group_field").lt(y_1)),
                            ),
                        )
                    })
                    .select([col("volume")])
                    .collect()
                    .unwrap();

                let v = count
                    .get(0)
                    .and_then(|v| v.get(0).map(|v| v.try_extract::<u32>().unwrap()))
                    .unwrap_or(0) as usize;

                debug!(
                    "wapuku: data_vec: (x, y)={:?}, (x_0, x_1)={:?}, (y_0, y_1)={:?}, v={:?}",
                    (x, y),
                    (x_0, x_1),
                    (y_0, y_1),
                    v
                );

                if v > 0 {
                    let group_box = Box::<dyn DataGroup>::from(Box::new(SimpleDataGroup::new(
                        v,
                        vec![],
                        DataBounds::XY(
                            property_x.to_range(Some(x_0), Some(x_1)),
                            property_y.to_range(Some(y_0), Some(y_1)),
                        ),
                    )));

                    data_vec[y][x].replace(group_box);
                }
            });
        });

        GroupsGrid::new(
            property_x.property().clone_to_box(),
            property_y.property().clone_to_box(),
            data_vec,
        )
    }

    fn build_summary(&self, frame_id: u128, column_op: Option<String>) -> Summary {
        //TODO
        // self.df.get_columns().len() < 20
        // self.df.lazy().filter()

        let desc = if let Some(column) = column_op {
            describe(
                &self
                    .df
                    .column(&column)
                    .expect("")
                    .clone()
                    .into_frame()
                    .lazy(),
            )
            .unwrap() //TODO describe_with_params
        } else {
            describe(&self.df.clone().lazy()).unwrap()
        };

        // debug!("desc={:?}", desc);
        Summary::new(
            wa_id(),
            frame_id,
            self.name.clone(),
            desc.get_columns()
                .into_iter()
                .enumerate()
                .skip(1)
                .map(|(i, c)| {
                    let name = c.name();

                    let dtype = self.df.column(name).expect(name).dtype();
                    // debug!("column={:?} type={:?} mean={:?}", c.name(), dtype, desc.get(2).map(|row|row.get(i).map(|v|format!("{}", v))));

                    let data_type = map_to_wapuku(dtype);
                    match data_type {
                        WapukuDataType::Numeric { .. } => {
                            // let min = desc.get(4).and_then(|row|row.get(i).map(|v|ToString::to_string(v))).unwrap_or(String::from("n/a"));
                            SummaryColumn::new(
                                String::from(name),
                                SummaryColumnType::Numeric {
                                    data: NumericColumnSummary::new(
                                        desc.get(4)
                                            .and_then(|row| {
                                                row.get(i).map(|v| ToString::to_string(v))
                                            })
                                            .unwrap_or(NA.into()),
                                        desc.get(2)
                                            .and_then(|row| {
                                                row.get(i).map(|v| ToString::to_string(v))
                                            })
                                            .unwrap_or(NA.into()),
                                        desc.get(8)
                                            .and_then(|row| {
                                                row.get(i).map(|v| ToString::to_string(v))
                                            })
                                            .unwrap_or(NA.into()),
                                    ),
                                },
                            )
                        }

                        WapukuDataType::String => {
                            let unique_values = self
                                .df
                                .column(name)
                                .and_then(|v| v.unique())
                                .map(|u| {
                                    u.rechunk()
                                        .iter()
                                        .take(3)
                                        .map(|v| ToString::to_string(&v))
                                        .collect::<Vec<String>>()
                                        .join(", ")
                                })
                                .unwrap_or(NA.into());

                            debug!("unique_values={:?}", unique_values);

                            SummaryColumn::new(
                                String::from(name),
                                SummaryColumnType::String {
                                    data: StringColumnSummary::new(unique_values),
                                },
                            )
                        }

                        WapukuDataType::Boolean => {
                            SummaryColumn::new(String::from(name), SummaryColumnType::Boolean)
                        }
                        WapukuDataType::Datetime => SummaryColumn::new(
                            String::from(name),
                            SummaryColumnType::Datetime {
                                data: NumericColumnSummary::new(
                                    desc.get(4)
                                        .and_then(|row| row.get(i).map(|v| ToString::to_string(v)))
                                        .unwrap_or(NA.into()),
                                    desc.get(2)
                                        .and_then(|row| row.get(i).map(|v| ToString::to_string(v)))
                                        .unwrap_or(NA.into()),
                                    desc.get(8)
                                        .and_then(|row| row.get(i).map(|v| ToString::to_string(v)))
                                        .unwrap_or(NA.into()),
                                ),
                            },
                        ),
                    }
                })
                .collect(),
            format!("{:?}", self.df.shape()),
        )
    }

    fn build_histogram(
        &self,
        frame_id: u128,
        column: String,
        bins: Option<usize>,
    ) -> Result<Histogram, WapukuError> {
        debug!("wapuku: build_histogram={:?}", column);

        match self.df.column(column.as_str())?.dtype() {
            // DataType::Boolean => {}
            DataType::UInt8
            | DataType::UInt16
            | DataType::UInt32
            | DataType::UInt64
            | DataType::Int8
            | DataType::Int16
            | DataType::Int32
            | DataType::Int64
            | DataType::Float32
            | DataType::Float64
            | DataType::Date
            | DataType::Time => Ok(self.group_by_numeric(frame_id, column, bins)?),
            // DataType::Decimal(_, _) => {}
            DataType::String => Ok(self.group_by_categoric(frame_id, column)?),
            // DataType::Binary => {}
            DataType::Datetime(_, _) => Ok(self.group_by_datetime(frame_id, column)?),
            // DataType::Duration(_) => {}
            // DataType::Array(_, _) => {}
            // DataType::List(_) => {}
            // DataType::Object(_) => {}
            // DataType::Null => {}
            // DataType::Categorical(_) => {}
            // DataType::Struct(_) => {}
            // DataType::Unknown => {}
            dtype => Err(WapukuError::DataLoad {
                msg: format!("can't build historgram for {} of type {}", column, dtype),
            }),
        }
    }

    fn fetch_data(
        &self,
        frame_id: u128,
        offset: usize,
        limit: usize,
    ) -> Result<DataLump, WapukuError> {
        let columns = self.df.get_columns();
        debug!("columns.len()={}", columns.len());

        columns.into_iter().enumerate().try_fold(DataLump::new(frame_id, offset, limit, columns.len()), |mut a, (col, s)| {
            debug!("col()={}", col);

            match s.dtype() {//TODO dtype into wdtype
                DataType::Boolean => {
                    a.add_column(WapukuDataType::Boolean, s.name())
                }

                DataType::UInt8 | DataType::UInt16 | DataType::UInt32 | DataType::UInt64 | DataType::Int8 | DataType::Int16 | DataType::Int32 | DataType::Int64 => {
                    a.add_column(WapukuDataType::Numeric, s.name())
                }

                DataType::Float32 | DataType::Float64 => {
                    a.add_column(WapukuDataType::Numeric, s.name())
                }

                DataType::String | DataType::Binary | DataType::Date | DataType::Datetime(_, _) | DataType::Duration(_) | DataType::Time | DataType::Array(_, _) | DataType::List(_) | /*DataType::Object | */
                DataType::Null | DataType::Categorical(_, _) | /*DataType::Struct(_) |*/   DataType::Unknown => {
                    a.add_column(WapukuDataType::String, s.name())
                }
                //TODO
                _ => {}
            };

            s.iter().skip(offset).take(limit).enumerate().map(|(row, v)| (row, v.to_string())).for_each(|(row, v)| {
                a.set_value(row, col, v);
            });

            Ok(a)
        })
    }

    fn apply_filter(&self, _frame_id: u128, filter: Filter) -> Result<FilteredFame, WapukuError> {
        debug!("apply_filter filter={:?}", filter);
        self.df
            .clone()
            .lazy()
            .filter(filter.into())
            .collect()
            .map(|df| FilteredFame::new(Box::new(PolarsData::new(df, String::from("filtered")))))
            .map_err(|e| WapukuError::DataLoad { msg: e.to_string() })
    }

    fn convert_column(
        &mut self,
        frame_id: u128,
        column: String,
        pattern: String,
    ) -> Result<SummaryColumn, WapukuError> {
        let res = self.df.apply(column.as_str(), move |str_val: &Series| {
            str_val
                .str()
                .unwrap()
                .into_iter()
                .map(|opt_name: Option<&str>| {
                    // opt_name.map(|v|chrono::NaiveDateTime::parse_from_str(v, "%Y-%m-%d %H:%M:%S").unwrap().timestamp_millis())
                    opt_name
                        .and_then(|v| {
                            debug!("convert_column: v={:?}", v);
                            match NaiveDateTime::parse_from_str(v, pattern.as_str()) {
                                Ok(v) => Some(v),

                                Err(_) => NaiveDate::parse_from_str(v, pattern.as_str())
                                    .map(|v| {
                                        NaiveDateTime::new(
                                            v,
                                            NaiveTime::parse_from_str("00:00", "%H:%M").unwrap(),
                                        )
                                    })
                                    .ok(),
                            }
                        })
                        .map(|v| v.timestamp_millis())
                })
                .collect::<Int64Chunked>()
                .into_datetime(TimeUnit::Milliseconds, None)
                .into_series()
        });
        self.build_summary(frame_id, Some(column))
            .columns()
            .first()
            .map(|c| c.clone())
            .ok_or(WapukuError::DataLoad { msg: "ups".into() })
    }

    fn clc_corrs(&self, frame_id: u128, columns: Vec<String>) -> Result<Corrs, WapukuError> {
        let mut corr_hash: HashMap<(String, String), f32> = HashMap::new();

        let mut l_df = self.df.clone().lazy();

        fn num_unique(vals: Series) -> Result<Option<Series>, PolarsError> {

            let unique_series = vals.unique()?.sort(false);

            let unique_vals_map = unique_series
                .iter()
                .enumerate()
                .fold(HashMap::new(), |mut m, (i, v)| {
                    m.insert(v, i);
                    m
                });

            return Ok(Some(Series::from_iter(vals.iter().map(|val| *unique_vals_map.get(&val).unwrap_or(&0) as f64))));
        }

        for column_name in columns.clone() {

            match self.df.column(&column_name)?.dtype() {
                DataType::String => {
                    debug!("clc_corrs: replace={:?}", column_name);

                    l_df = l_df.clone().with_column(
                        col(&column_name)
                            .apply(move |s| num_unique(s), GetOutput::from_type(DataType::Float64)),
                    );

                }

                _ => {},
            }
        }

        debug!("l_df={:?}", l_df.clone().collect());

        for (i, column_0) in columns.iter().enumerate() {
            //TODO itertools
            for column_1 in columns.iter().skip(i + 1) {
                debug!(
                    "clc_corrs: column_0={:?}, column_1={:?}",
                    column_0, column_1
                );
                let expr = pearson_corr(
                    Expr::Column(column_0.as_str().into()),
                    Expr::Column(column_1.as_str().into()),
                    0,
                )
                .alias("corr");
                let res = l_df.clone().select(&[expr]).collect();

                corr_hash.insert(
                    (column_0.clone(), column_1.clone()),
                    res.as_ref()?.column("corr")?.f64()?.get(0).expect("corr") as f32,
                );
            }
        }

        debug!("corr_hash={:?}", corr_hash);

        Ok(Corrs::new(frame_id, columns, corr_hash))
    }
}

impl From<Filter> for Expr {
    fn from(filter: Filter) -> Self {
        if let Some(conditions) = filter.conditions() {
            conditions_to_expr(conditions)
        } else {
            Expr::Wildcard
        }
    }
}

fn conditions_to_expr(condition: &ConditionType) -> Expr {
    match condition {
        ConditionType::Single {
            column_name,
            condition,
        } => match condition {
            Condition::Numeric { min, max } => col(column_name)
                .gt_eq(*min)
                .and(col(column_name).lt_eq(*max)),
            Condition::String { pattern } => col(column_name)
                .str()
                .contains_literal(lit(pattern.to_owned())),
            Condition::Boolean { val } => col(column_name).eq(*val),
        },
        ConditionType::Compoiste { conditions, ctype } => {
            if conditions.len() == 0 {
                warn!("empty composit condition");
                Expr::Wildcard
            } else if conditions.len() == 1 {
                conditions_to_expr(conditions.get(0).unwrap())
            } else {
                conditions.into_iter().skip(1).fold(
                    conditions_to_expr(conditions.get(0).expect("no first condition")),
                    |a, c| match ctype {
                        CompositeType::AND => a.and(conditions_to_expr(c)),
                        CompositeType::OR => a.or(conditions_to_expr(c)),
                    },
                )
            }
        }
    }
}

#[cfg(test)]
mod filter_test {
    use crate::model::{Condition, ConditionType, Filter};
    use crate::polars_df::tests::dummy_filter;
    use log::debug;
    use polars::prelude::Expr;

    #[test]
    pub fn test_string_pattern() {
        let mut filter = dummy_filter();
        filter.add_condition(
            ConditionType::Single {
                column_name: "property_2".into(),
                condition: Condition::String {
                    pattern: "aaaa".into(),
                },
            },
            None,
        );

        let expr: Expr = filter.into();

        println!("expr={:?}", expr);

        assert_eq!(
            "col(\"property_2\").str.contains([Utf8(aaaa)])",
            format!("{:?}", expr)
        );
    }

    #[test]
    pub fn test_num_pattern() {
        let mut filter = dummy_filter();
        filter.add_condition(
            ConditionType::Single {
                column_name: "property_1".into(),
                condition: Condition::Numeric {
                    min: -10.0,
                    max: 10.0,
                },
            },
            None,
        );

        let expr: Expr = filter.into();

        println!("expr={:?}", expr);

        assert_eq!(
            "[([(col(\"property_1\")) >= (-10.0)]) & ([(col(\"property_1\")) <= (10.0)])]",
            format!("{:?}", expr)
        );
    }

    #[test]
    pub fn test_bool_pattern() {
        let mut filter = dummy_filter();
        filter.add_condition(
            ConditionType::Single {
                column_name: "property_3".into(),
                condition: Condition::Boolean { val: true },
            },
            None,
        );

        let expr: Expr = filter.into();

        println!("expr={:?}", expr);

        assert_eq!("[(col(\"property_3\")) == (true)]", format!("{:?}", expr));
    }

    #[test]
    pub fn test_and() {
        let mut filter = dummy_filter();
        filter.add_condition(
            ConditionType::Single {
                column_name: "property_2".into(),
                condition: Condition::String {
                    pattern: "aaaa".into(),
                },
            },
            None,
        );
        filter.add_condition(
            ConditionType::Single {
                column_name: "property_2".into(),
                condition: Condition::String {
                    pattern: "bbb".into(),
                },
            },
            None,
        );

        let expr: Expr = filter.into();

        assert_eq!("[(col(\"property_2\").str.contains([Utf8(aaaa)])) & (col(\"property_2\").str.contains([Utf8(bbb)]))]", format!("{:?}", expr));
    }

    #[test]
    pub fn test_edit_top() {
        let mut filter = dummy_filter();
        let condition_type_0 = ConditionType::Single {
            column_name: "property_2".into(),
            condition: Condition::String {
                pattern: "aaaa".into(),
            },
        };

        filter.add_condition(condition_type_0, None);

        let condition_type_0_ptr = filter.top_condition_ptr().expect("top_condition_ptr");

        let Some(ConditionType::Single {
            column_name,
            condition: Condition::String { pattern },
        }) = filter.conditions()
        else {
            panic!("no conditions")
        };

        assert_eq!(column_name, "property_2");
        assert_eq!(pattern, "aaaa");

        let condition_type_1 = ConditionType::Single {
            column_name: "property_2".into(),
            condition: Condition::String {
                pattern: "bbb".into(),
            },
        };

        filter.add_condition(condition_type_1, Some(condition_type_0_ptr));

        let Some(ConditionType::Single {
            column_name,
            condition: Condition::String { pattern },
        }) = filter.conditions()
        else {
            panic!("no conditions")
        };

        assert_eq!(column_name, "property_2");
        assert_eq!(pattern, "bbb");
    }

    #[test]
    pub fn test_edit_sub_1() {
        let mut filter = dummy_filter();
        let condition_type_0 = ConditionType::Single {
            column_name: "property_1".into(),
            condition: Condition::String {
                pattern: "aaaa".into(),
            },
        };
        let condition_type_1 = ConditionType::Single {
            column_name: "property_2".into(),
            condition: Condition::String {
                pattern: "bbbb".into(),
            },
        };

        filter.add_condition(condition_type_0, None);
        filter.add_condition(condition_type_1, None);

        let Some(ConditionType::Compoiste { conditions, ctype }) = filter.conditions() else {
            panic!("no conditions")
        };
        let condition_type_0_f = conditions.get(0).expect("condition_type_1");
        let condition_type_0_f_ptr = condition_type_0_f as *const _;
        let condition_type_1_f = conditions.get(1).expect("condition_type_1");
        let condition_type_1_f_ptr = condition_type_1_f as *const _;

        let condition_type_1 = ConditionType::Single {
            column_name: "property_2".into(),
            condition: Condition::String {
                pattern: "cccc".into(),
            },
        };

        filter.add_condition(condition_type_1, Some(condition_type_1_f_ptr));

        let Some(ConditionType::Compoiste { conditions, ctype }) = filter.conditions() else {
            panic!("no conditions")
        };
        let ConditionType::Single {
            column_name,
            condition: Condition::String { pattern },
        } = conditions.get(1).expect("condition_type_1").clone()
        else {
            panic!("no condition_type_1")
        };

        assert_eq!(column_name, "property_2");
        assert_eq!(pattern, "cccc");
    }
}

// impl Drop for PolarsData {
//     fn drop(&mut self) {
//         debug!("wapuku: Drop for PolarsData")
//     }
// }

fn map_to_wapuku(d_type: &DataType) -> WapukuDataType {
    //TODO
    match d_type {
        DataType::String => WapukuDataType::String,

        DataType::Float64
        | DataType::Float32
        | DataType::Int8
        | DataType::Int16
        | DataType::Int32
        | DataType::Int64
        | DataType::UInt8
        | DataType::UInt16
        | DataType::UInt32
        | DataType::UInt64 => WapukuDataType::Numeric,

        DataType::Boolean => WapukuDataType::Boolean,

        DataType::Datetime(..) | DataType::Date => WapukuDataType::Datetime,
        _ => WapukuDataType::String,
    }
}

pub fn load_zip(data: Box<Vec<u8>>) -> Result<Vec<(DataFrame, String)>, WapukuError> {
    let mut archive = ZipArchive::new(Cursor::new(data.as_slice()))?;
    archive
        .file_names()
        .map(|s| String::from(s))
        .collect::<Vec<String>>()
        .into_iter()
        .map(|file| {
            let mut bytes = Vec::new();
            archive.by_name(file.as_str())?.read_to_end(&mut bytes)?;

            if file.ends_with("csv") {
                load_csv(Box::new(bytes)).map(|df| (df, file))
            } else if file.ends_with("parquet") {
                load_parquet(Box::new(bytes)).map(|df| (df, file))
            } else {
                Err(WapukuError::DataLoad {
                    msg: format!("Unexepcted file ending {}", file),
                })
            }
        })
        .collect::<Result<Vec<(DataFrame, String)>, WapukuError>>()
}

pub fn load_csv(csv_bytes: Box<Vec<u8>>) -> Result<DataFrame, WapukuError> {
    CsvReader::new(Cursor::new(csv_bytes.as_slice()))
        .finish()
        .map_err(|e| e.into())
}

pub fn load_parquet(parquet_bytes: Box<Vec<u8>>) -> Result<DataFrame, WapukuError> {
    ParquetReader::new(Cursor::new(parquet_bytes.as_slice()))
        .finish()
        .map_err(|e| e.into())
}

pub(crate) fn group_by_1<E: AsRef<[Expr]>>(
    df: &DataFrame,
    group_by_field: &str,
    step: i64,
    aggregations: E,
    offset: i64,
) -> WapukuResult<DataFrame> {
    let df = df.sort([group_by_field], vec![true], false)?;

    let df = df
        .clone()
        .lazy()
        .sort(group_by_field, SortOptions::default())
        .group_by_dynamic(
            col(group_by_field.into()),
            [],
            DynamicGroupOptions {
                index_column: group_by_field.into(),
                every: Duration::new(step),
                period: Duration::new(step),
                offset: Duration::new(offset),
                include_boundaries: true,
                closed_window: ClosedWindow::Left,
                start_by: WindowBound,
                check_sorted: true,
                label: Label::DataPoint,
            },
        )
        .agg(aggregations)
        // .agg([
        //     col("property_3").count().alias("volume")
        // ])
        .collect()?;

    let mut df = df.sort([group_by_field], vec![true], false)?;

    df.rename(group_by_field, "group_by_field")
        .expect("rename group_by_field");
    // let df = df.clone().lazy().with_column(lit(1).alias("primary_field_group")).collect()?;
    // df.rename(group_by_field, "secondary_group_field");

    debug!("wapuku: df grouped={:?}", df);

    Ok(df)
}

pub(crate) fn group_by_2<E: AsRef<[Expr]>>(
    df: &DataFrame,
    primary_group_by_field: &str,
    primary_step: i64,
    secondary_group_by_field: &str,
    secondary_step: i64,
    aggregations: E,
    primary_offset: i64,
    secondary_offset: i64,
) -> WapukuResult<DataFrame> {
    let primary_field_value = "primary_field_value";
    let primary_field_group = "primary_field_group"; //expanded value of the field in the second column group, to be joined back to main frame

    let primary_field_grouped_and_expanded = df.clone()
        .lazy()
        .select(&[
            col(primary_group_by_field).alias(primary_field_group)
        ])
        .sort(primary_field_group, Default::default())
        .group_by_dynamic(
            col(primary_field_group.into()),
            [],
            DynamicGroupOptions {
                index_column: primary_field_group.into(),
                every: Duration::new(primary_step),
                period: Duration::new(primary_step),
                offset: Duration::new(primary_offset),
                include_boundaries: true,
                closed_window: ClosedWindow::Left,
                start_by: WindowBound,
                check_sorted: true,
                label: Label::DataPoint,
            },
        ).agg([
        col(primary_field_group).alias(primary_field_value)
    ])/*.with_row_count("primary_index", None)*/
        .explode([primary_field_value]).collect()?; //primary_field_in_group not used, for debug

    debug!(
        "wapuku: primary_field_grouped_and_expanded={:?}",
        primary_field_grouped_and_expanded
    );
    let mut df = df.sort([primary_group_by_field], vec![true], false)?;

    let df = df
        .with_column(primary_field_grouped_and_expanded.column(primary_field_group).unwrap().clone())?
        // .with_column(primary_field_grouped_and_expanded.column("primary_index").unwrap().clone())?
        // .with_column({
        //     let mut series = primary_field_grouped_and_expanded.column("_lower_boundary").unwrap().clone();
        //     series.rename("primary_lower_boundary");
        //     series
        // })?
        ;
    let df = df.sort([secondary_group_by_field], vec![true], false)?;

    // let mut df = df.left_join(&primary_field_grouped_and_expanded, [primary_group_by_field], ["primary_field_value"] )?;

    debug!("2. wapuku: df={:?}", df);

    let mut df = df
        .clone()
        .lazy()
        .sort(secondary_group_by_field, Default::default())
        .group_by_dynamic(
            col(secondary_group_by_field.into()),
            [col(primary_field_group)],
            DynamicGroupOptions {
                index_column: secondary_group_by_field.into(),
                every: Duration::new(secondary_step),
                period: Duration::new(secondary_step),
                offset: Duration::new(secondary_offset),
                include_boundaries: true,
                closed_window: ClosedWindow::Left,
                start_by: WindowBound,
                check_sorted: true,
                label: Label::DataPoint,
            },
        )
        .agg(aggregations)
        // .agg([
        //     col("property_3").count().alias("volume")
        // ])
        .collect()?;

    df.rename(secondary_group_by_field, "secondary_group_field")
        .expect("rename secondary_group_field");

    debug!("wapuku: df grouped={:?}", df);

    Ok(df)
}

pub fn describe(lf: &LazyFrame) -> PolarsResult<DataFrame> {
    describe_with_params(
        lf,
        true,
        vec![
            (
                "count".to_owned(),
                Box::new(|dt: &DataType| dt.is_ord()) as Box<dyn Fn(&DataType) -> bool>,
                Box::new(|name: &String| {
                    col(name).count().alias(format!("{} count", name).as_str())
                }) as Box<dyn Fn(&String) -> Expr>,
            ),
            (
                "null_count".to_owned(),
                Box::new(|dt: &DataType| dt.is_ord()) as Box<dyn Fn(&DataType) -> bool>,
                Box::new(|name: &String| {
                    col(name)
                        .null_count()
                        .alias(format!("{} null_count", name).as_str())
                }) as Box<dyn Fn(&String) -> Expr>,
            ),
            (
                "mean".to_owned(),
                Box::new(|dt: &DataType| dt.is_numeric()) as Box<dyn Fn(&DataType) -> bool>,
                Box::new(|name: &String| col(name).mean().alias(format!("{} mean", name).as_str()))
                    as Box<dyn Fn(&String) -> Expr>,
            ),
            (
                "std".to_owned(),
                Box::new(|dt: &DataType| dt.is_numeric()) as Box<dyn Fn(&DataType) -> bool>,
                Box::new(|name: &String| col(name).std(1).alias(format!("{} std", name).as_str()))
                    as Box<dyn Fn(&String) -> Expr>,
            ),
            (
                "min".to_owned(),
                Box::new(|dt: &DataType| dt.is_ord()) as Box<dyn Fn(&DataType) -> bool>,
                Box::new(|name: &String| col(name).min().alias(format!("{} min", name).as_str()))
                    as Box<dyn Fn(&String) -> Expr>,
            ),
            (
                "25%".to_owned(),
                Box::new(|dt: &DataType| dt.is_numeric()) as Box<dyn Fn(&DataType) -> bool>,
                Box::new(move |name: &String| {
                    col(name)
                        .quantile(lit(0.25), QuantileInterpolOptions::Nearest)
                        .alias(format!("0.25 {}", name).as_str())
                }) as Box<dyn Fn(&String) -> Expr>,
            ),
            (
                "50%".to_owned(),
                Box::new(|dt: &DataType| dt.is_numeric()) as Box<dyn Fn(&DataType) -> bool>,
                Box::new(move |name: &String| {
                    col(name)
                        .quantile(lit(0.5), QuantileInterpolOptions::Nearest)
                        .alias(format!("0.5 {}", name).as_str())
                }) as Box<dyn Fn(&String) -> Expr>,
            ),
            (
                "75%".to_owned(),
                Box::new(|dt: &DataType| dt.is_numeric()) as Box<dyn Fn(&DataType) -> bool>,
                Box::new(move |name: &String| {
                    col(name)
                        .quantile(lit(0.75), QuantileInterpolOptions::Nearest)
                        .alias(format!("0.75 {}", name).as_str())
                }) as Box<dyn Fn(&String) -> Expr>,
            ),
            (
                "max".to_owned(),
                Box::new(|dt: &DataType| dt.is_ord()) as Box<dyn Fn(&DataType) -> bool>,
                Box::new(|name: &String| col(name).max().alias(format!("{} max", name).as_str()))
                    as Box<dyn Fn(&String) -> Expr>,
            ),
        ],
        None,
    )
}

type DescribeAggParam = (
    String,                         //name
    Box<dyn Fn(&DataType) -> bool>, //when applicable
    Box<dyn Fn(&String) -> Expr>,   //aggregation expression
);

pub fn describe_with_params(
    lf: &LazyFrame,
    fields_to_header: bool,
    aggs: Vec<DescribeAggParam>,
    only_columns: Option<Vec<&str>>,
) -> PolarsResult<DataFrame> {
    let exprs = lf
        .schema()?
        .iter()
        .map(|(name, dt)| (name.to_string(), dt))
        .filter(|(name, _)| {
            only_columns
                .as_ref()
                .map_or(true, |v| v.contains(&name.as_str()))
        })
        .enumerate()
        .flat_map(|(i, (name, dt))| {
            once(Expr::Literal(LiteralValue::String(name.to_string())).alias(name.as_str()))
                .chain(aggs.iter().enumerate().map(|(ai, a)| {
                    if a.1(dt) {
                        a.2(&name)
                    } else {
                        lit(NULL)
                            .cast(dt.clone())
                            .alias(format!("{} {}", i, ai).as_str()) //i and ai to keep names unique
                    }
                }))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    let columns_len = exprs.len();

    let mut summary_df = lf.clone().select(exprs).collect()?;

    let aggs_len = aggs.len() + 1;
    let index = (0..columns_len / aggs_len)
        .flat_map(|i| std::iter::repeat(i as i32).take(aggs_len))
        .collect::<Vec<i32>>();

    summary_df = summary_df.transpose(None, None)?;

    summary_df.insert_column(
        0,
        Series::new(
            "columns",
            once("name".to_owned())
                .chain(aggs.iter().map(|agg| agg.0.clone()))
                .cycle()
                .take(columns_len)
                .collect::<Vec<String>>(),
        ),
    )?;
    summary_df.insert_column(0, Series::new("index", index))?;

    summary_df = pivot::pivot(
        &summary_df,
        ["column_0"],
        ["index"],
        ["columns"],
        false,
        None,
        None,
    )?;

    if !fields_to_header {
        summary_df = summary_df
            .clone()
            .lazy()
            .select(
                once("name")
                    .chain(aggs.iter().map(|agg| agg.0.as_str()))
                    .enumerate()
                    .map(|(i, name)| {
                        if i > 0 {
                            col(name).cast(DataType::Float64)
                        } else {
                            col(name)
                        }
                    })
                    .collect::<Vec<Expr>>(),
            )
            .collect()?;

        Ok(summary_df)
    } else {
        summary_df = summary_df.drop("index")?;

        summary_df = summary_df.transpose(None, Some(Either::Left("name".to_owned())))?;

        summary_df = summary_df
            .clone()
            .lazy()
            .select(
                lf.schema()?
                    .iter()
                    .map(|(name, dt)| {
                        if dt.is_numeric() {
                            col(name.as_str()).cast(DataType::Float64)
                        } else {
                            col(name.as_str())
                        }
                    })
                    .collect::<Vec<Expr>>(),
            )
            .collect()?;

        summary_df.insert_column(
            0,
            Series::new(
                "describe",
                aggs.iter().map(|q| q.0.clone()).collect::<Vec<String>>(),
            ),
        )?;

        Ok(summary_df)
    }
}

#[cfg(test)]
pub(super) mod tests {
    #![feature(async_fn_in_trait)]

    use std::collections::HashSet;
    use std::fs::File;
    use std::iter;

    use log::debug;
    use polars::datatypes::AnyValue::List;
    use polars::df;
    use polars::export::arrow::compute::filter::filter;
    use polars::export::arrow::io::ipc::read::FileReader;
    use polars::export::chrono;
    use polars::export::chrono::{NaiveDate, NaiveDateTime};
    use polars::prelude::LiteralValue::DateTime;
    use polars::prelude::*;
    // use polars::io::prelude::utils::
    use polars::io::mmap::MmapBytesReader;

    use crate::data_type::{WapukuDataType, WapukuDataValues};
    use crate::model::{
        CompositeType, Condition, ConditionType, Data, DataGroup, DataProperty, Filter, GroupsGrid,
        NumericColumnSummary, Property, PropertyRange, StringColumnSummary, Summary, SummaryColumn,
        SummaryColumnType,
    };
    use crate::polars_df::{describe, describe_with_params, group_by_2, PolarsData};
    use crate::tests::init_log;

    #[ctor::ctor]
    fn init() {
        std::env::set_var("POLARS_FMT_MAX_ROWS", "100");
        std::env::set_var("FMT_MAX_COLS", "1000");

        init_log();
    }

    #[test]
    fn test_build_summary_ints() {
        let df = df!(
            "property_1" => &[1,   2,   3,   1,   2,   3,   1,   2,   3,],
            "property_2" => &[10,  10,  10,  20,  20,  20,  30,  30,  30,],
            "property_3" => &[11,  12,  13,  21,  22,  23,  31,  32,  33,]
        )
        .unwrap();
        let mut data = PolarsData::new(df, String::from("test"));

        let summary = data.build_summary(0, None);

        check_numeric_column(&summary, 0, "1.0", "2.0", "3.0");
        check_numeric_column(&summary, 1, "10.0", "20.0", "30.0");
    }

    #[test]
    fn test_build_summary_dates() {
        let date_series = DatetimeChunked::from_naive_datetime(
            "days",
            vec![
                chrono::NaiveDateTime::parse_from_str("2023-01-01 00:00:01", "%Y-%m-%d %H:%M:%S")
                    .unwrap(),
                chrono::NaiveDateTime::parse_from_str("2023-01-15 00:00:01", "%Y-%m-%d %H:%M:%S")
                    .unwrap(),
                chrono::NaiveDateTime::parse_from_str("2023-02-01 00:00:02", "%Y-%m-%d %H:%M:%S")
                    .unwrap(),
                chrono::NaiveDateTime::parse_from_str("2023-02-15 00:00:02", "%Y-%m-%d %H:%M:%S")
                    .unwrap(),
                chrono::NaiveDateTime::parse_from_str("2023-03-01 00:00:02", "%Y-%m-%d %H:%M:%S")
                    .unwrap(),
                chrono::NaiveDateTime::parse_from_str("2023-03-15 00:00:02", "%Y-%m-%d %H:%M:%S")
                    .unwrap(),
            ],
            TimeUnit::Milliseconds,
        )
        .into_series();
        // let int_series = Int64Chunked::from_vec("ints", vec![1, 2, 3, 4, 5, 6]).into_series();

        debug!("date_series={:?}", date_series.dtype());

        let df = DataFrame::new(vec![date_series]).unwrap();

        // let df = ParquetReader::new(File::open("../wapuku-egui/www/data/userdata1.parquet").unwrap()).finish().unwrap();

        let mut data = PolarsData::new(df, String::from("test"));

        let summary = data.build_summary(0, None);

        println!("summary={:?}", summary);

        if let SummaryColumnType::Datetime { data } = summary.columns()[0].dtype() {
        } else {
            panic!("column is not date: {:?}", summary.columns()[0].dtype())
        }

        // check_numeric_column(&summary, 0, "1.0", "2.0", "3.0");
        // check_numeric_column(&summary, 1, "10.0", "20.0", "30.0");
    }

    #[test]
    fn test_build_histogram_str() {
        let df = df!(
            "property_1" => &[1i32,   2i32,    3i32,  1i32,   2i32,    3i32,    1i32,    2i32,   3i32],
            "property_2" => &[1f32,   2f32,    3f32,  1f32,    2f32,   3f32,    1f32,    2f32,   3f32],
            "property_3" => &["A",  "B",  "C",  "A",  "B",  "C",  "C",  "B", "C"]
        ).unwrap();

        let mut data = PolarsData::new(df, String::from("test"));

        let histogram = data
            .build_histogram(0u128, String::from("property_3"), None)
            .expect("build_histogram");

        let y = histogram.values();
        println!("y={:?}", y);
        assert_eq!(y.get(0).unwrap().1, 2);
        assert_eq!(y.get(1).unwrap().1, 3);
        assert_eq!(y.get(2).unwrap().1, 4);
    }

    #[test]
    fn test_build_date_histogram_f32() {
        let df = DataFrame::new(vec![DatetimeChunked::from_naive_datetime(
            "registration_dttm",
            vec![
                chrono::NaiveDateTime::parse_from_str("2023-01-01 00:00:01", "%Y-%m-%d %H:%M:%S")
                    .unwrap(),
                chrono::NaiveDateTime::parse_from_str("2023-01-15 00:00:01", "%Y-%m-%d %H:%M:%S")
                    .unwrap(),
                chrono::NaiveDateTime::parse_from_str("2023-02-01 00:00:02", "%Y-%m-%d %H:%M:%S")
                    .unwrap(),
                chrono::NaiveDateTime::parse_from_str("2023-02-15 00:00:02", "%Y-%m-%d %H:%M:%S")
                    .unwrap(),
                chrono::NaiveDateTime::parse_from_str("2023-03-01 00:00:02", "%Y-%m-%d %H:%M:%S")
                    .unwrap(),
                chrono::NaiveDateTime::parse_from_str("2023-03-15 00:00:02", "%Y-%m-%d %H:%M:%S")
                    .unwrap(),
            ],
            TimeUnit::Milliseconds,
        )
        .into_series()])
        .unwrap();

        // let df = ParquetReader::new(File::open("../wapuku-egui/www/data/userdata1.parquet").unwrap()).finish().unwrap();

        println!("{:?}", df);

        let mut data = PolarsData::new(df, String::from("test"));

        let summary = data.build_summary(0u128, None);
        println!("summary={:?}", summary);

        let histogram = data
            .build_histogram(0u128, String::from("registration_dttm"), None)
            .expect("build_histogram");

        println!("histogram={:?}", histogram);
        assert_eq!(histogram.values()[0].1, 1);
    }

    #[test]
    fn test_convert_date() {
        let v = NaiveDateTime::parse_from_str("7/9/1972 00:00:00", "%m/%d/%Y %T");
        debug!("v={:?}", v);

        let v = NaiveDate::parse_from_str("7/9/1972", "%m/%d/%Y");
        debug!("v={:?}", v);
    }

    #[test]
    fn test_convert_column_utf8_to_date_time() {
        let df = df!(
            "days" => &["2023-01-01 00:00:01",  "2023-01-15 00:00:01",  "2023-02-01 00:00:02"]
        )
        .unwrap();

        let mut data = PolarsData::new(df, String::from("test"));

        println!("1. {:?}", data.build_summary(0, None).columns()[0].dtype());

        let ok = data
            .convert_column(0u128, "days".into(), "%Y-%m-%d %T".into())
            .expect("convert_column");
        println!("ok={:?}", ok);
        println!("2. {:?}", data.build_summary(0, None).columns()[0].dtype());
        println!("2. {:?}", data.build_histogram(0, "days".into(), None));

        // assert!(ok);
    }

    #[test]
    fn test_convert_column_utf8_to_date() {
        let df = df!(
            "days" => &["01/01/2023",  "01/15/2023",  "02/01/2023"]
        )
        .unwrap();

        let mut data = PolarsData::new(df, String::from("test"));

        println!("1. {:?}", data.build_summary(0, None).columns()[0].dtype());

        let ok = data
            .convert_column(0u128, "days".into(), "%m/%d/%Y".into())
            .expect("convert_column");

        println!("ok={:?}", ok);
        println!("2. {:?}", data.build_summary(0, None).columns()[0].dtype());

        // assert!(ok);
    }

    #[test]
    fn test_build_histogram_f32() {
        let df = df!(
            "property_1" => &[1i32,   2i32,    3i32,  1i32,   2i32,    3i32,    1i32,    2i32,   3i32],
            "property_2" => &[1f32,   2f32,    3f32,  1f32,    2f32,   3f32,    1f32,    2f32,   3f32],
            "property_3" => &["A",  "B",  "C",  "A",  "B",  "C",  "C",  "B", "C"]
        ).unwrap();

        let mut data = PolarsData::new(df, String::from("test"));

        let histogram = data
            .build_histogram(0u128, String::from("property_2"), Some(10))
            .expect("build_histogram");

        println!("histogram={:?}", histogram);

        let y = histogram.values();
        println!("y={:?}", y);
        assert_eq!(y.get(0).unwrap().0, "(-inf, 0.00]");
        assert_eq!(y.get(0).unwrap().1, 0);
        assert_eq!(y.get(3).unwrap().0, "(0.80, 1.20]");
        assert_eq!(y.get(3).unwrap().1, 3);

        // let histogram = data.build_histogram(0u128, String::from("property_3"), Some(10)).expect("build_histogram");
        //
        // println!("histogram={:?}", histogram);
    }

    #[test]
    fn test_build_summary_str() {
        let df = df!(
            "property_1" => &[1i32,   2i32,    3i32,  1i32,   2i32,    3i32,    1i32,    2i32,   3i32],
            "property_2" => &[1f32,   2f32,    3f32,  1f32,    2f32,   3f32,    1f32,    2f32,   3f32],
            "property_3" => &["A",  "B",  "C",  "D",  "E",  "F",  "G",  "H", "I"]
        ).unwrap();
        let mut data = PolarsData::new(df, String::from("test"));

        let summary = data.build_summary(0, None);

        check_numeric_column(&summary, 0, "1.0", "2.0", "3.0");
    }

    #[test]
    fn test_apply_filter() {
        let df = df!(
            "property_1" => &[1i32,   2i32,    3i32,   1i32,   2i32,   3i32,    1i32,    2i32,   3i32],
            "property_2" => &[1f32,   2f32,    3f32,   1f32,   2f32,   3f32,    1f32,    2f32,   3f32],
            "property_3" => &["AB1",  "AB2",   "BC1",  "BC1",  "E",    "F",     "G",     "H",    "I"]
        ).unwrap();
        let mut data = PolarsData::new(df, String::from("test"));

        let mut filter = dummy_filter();

        filter.add_condition(
            ConditionType::Compoiste {
                conditions: vec![
                    ConditionType::Compoiste {
                        conditions: vec![
                            ConditionType::Single {
                                column_name: "property_3".into(),
                                condition: Condition::String {
                                    pattern: "A".into(),
                                },
                            },
                            ConditionType::Single {
                                column_name: "property_2".into(),
                                condition: Condition::Numeric { min: 1.0, max: 1.0 },
                            },
                        ],
                        ctype: CompositeType::AND,
                    },
                    ConditionType::Compoiste {
                        conditions: vec![
                            ConditionType::Single {
                                column_name: "property_3".into(),
                                condition: Condition::String {
                                    pattern: "C".into(),
                                },
                            },
                            ConditionType::Single {
                                column_name: "property_1".into(),
                                condition: Condition::Numeric { min: 3.0, max: 3.0 },
                            },
                        ],
                        ctype: CompositeType::AND,
                    },
                ],
                ctype: CompositeType::OR,
            },
            None,
        );

        // filter.add_condition(ConditionType::Single {column_name: "property_3".into(), condition: Condition::String {pattern:"B".into()}}, None);

        let filtered_frame = data.apply_filter(0u128, filter);
        let summary = filtered_frame.unwrap().data().build_summary(0u128, None);
        let columns = summary.columns();
        let property_3_column = columns.get(2).expect("property_3_column");
        let SummaryColumnType::String { data } = property_3_column.dtype() else {
            panic!("no property_3_column data")
        };

        // debug!("data.unique_values()={:?}", data.unique_values());

        assert_eq!("\"AB1\", \"BC1\"", data.unique_values())
    }

    pub(crate) fn dummy_filter() -> Filter {
        Filter::empty(
            0u128,
            vec![
                SummaryColumn::new(
                    "property_1",
                    SummaryColumnType::Numeric {
                        data: NumericColumnSummary::new("0", "1", "2"),
                    },
                ),
                SummaryColumn::new(
                    "property_2",
                    SummaryColumnType::String {
                        data: StringColumnSummary::new("aaa bbb ccc"),
                    },
                ),
                SummaryColumn::new("property_3", SummaryColumnType::Boolean),
            ],
        )
    }

    #[test]
    fn test_fetch_data() {
        // let column_vec = iter::repeat(3).map(|_|Some(String::from("XXX"))).collect::<Vec<Option<String>>>();

        let column_vec = (0..3)
            .map(|_| Some(String::from("XXX")))
            .collect::<Vec<Option<String>>>();

        let mut df = df!(
            "property_1" => &[1i32,   2i32,   3i32,  10i32,    20i32,   30i32,    100i32,    200i32,   300i32],
            "property_2" => &[1f32,   2f32,   3f32,  10f32,    20f32,   30f32,    100f32,    200f32,   300f32],
            "property_3" => &["A",    "B",    "C",   "D",      "E",     "F",      "G",       "H",      "I"]
        ).unwrap();
        let mut data = PolarsData::new(df, String::from("test"));

        let data = data.fetch_data(0u128, 3, 3).unwrap();
        debug!("data={:?}", data.columns()[0]);

        assert_eq!(
            data.data()[0],
            vec![
                Some(String::from("10")),
                Some(String::from("10.0")),
                Some(String::from("\"D\""))
            ]
        );

        // println!("alasdfsafsa");
    }

    fn check_numeric_column(summary: &Summary, i: usize, min: &str, avg: &str, max: &str) {
        if let SummaryColumnType::Numeric { data } = summary.columns()[i].dtype() {
            assert_eq!(data.min(), min);
            assert_eq!(data.avg(), avg);
            assert_eq!(data.max(), max);
        } else {
            panic!(
                "column {} is not numeric: {:?}",
                i,
                summary.columns()[i].dtype()
            )
        }
    }

    #[test]
    fn test_polars_data_1x3x3() {
        /**
         property_1     1   2   3    - X
         property_2
            1           11  12  13
            2           21  22  23
            3           31  32  33

            Y

        **/
        let mut df = df!(
            "property_1" => &[1,   2,   3,   1,   2,   3,   1,   2,   3,],
            "property_2" => &[10,  10,  10,  20,  20,  20,  30,  30,  30,],
            "property_3" => &[11,  12,  13,  21,  22,  23,  31,  32,  33,]
        )
        .unwrap();

        debug!("wapuku: df: {:?}", df);

        // let mut grid = x_property_1_y_property_2_to_3_x_3_data(df, (Some(1i64), Some(4i64)), (Some(10i64), Some(31i64)));
        let grid = x_property_1_y_property_2_to_3_x_3_data(df, (None, None), (None, None));

        debug!("wapuku: grid: {:?}", grid);

        // let data = grid.data();

        assert_eq!(grid.group_at(0, 0).unwrap().volume(), 1);
        assert_eq!(grid.group_at(0, 1).unwrap().volume(), 1);
        assert_eq!(grid.group_at(0, 2).unwrap().volume(), 1);

        assert_eq!(grid.group_at(1, 0).unwrap().volume(), 1);
        assert_eq!(grid.group_at(1, 1).unwrap().volume(), 1);
        assert_eq!(grid.group_at(1, 2).unwrap().volume(), 1);

        assert_eq!(grid.group_at(2, 0).unwrap().volume(), 1);
        assert_eq!(grid.group_at(2, 1).unwrap().volume(), 1);
        assert_eq!(grid.group_at(2, 2).unwrap().volume(), 1);
    }

    #[test]
    fn test_polars_data_1x1x3() {
        /**
         property_1     1   2   3    - X
         property_2
            1           11
            2           21
            3           31

            Y

        **/
        let mut df = df!(
            "property_1" => &[1,   1,   1,  ],
            "property_2" => &[10,  20,  30, ],
            "property_3" => &[11,  21,  31, ]
        )
        .unwrap();

        debug!("wapuku: df: {:?}", df);

        let grid = x_property_1_y_property_2_to_3_x_3_data(
            df,
            (Some(1_i64), Some(1_i64)),
            (Some(10i64), Some(31i64)),
        );
        // let mut grid = x_property_1_y_property_2_to_3_x_3_data(df, (None, None), (None, None));

        debug!("wapuku: grid: {:?}", grid);

        // let data = grid.data();

        assert_eq!(grid.group_at(0, 0).unwrap().volume(), 1);
        assert_eq!(grid.group_at(0, 1).unwrap().volume(), 1);
        assert_eq!(grid.group_at(0, 2).unwrap().volume(), 1);

        assert_eq!(grid.group_at(1, 0).unwrap().volume(), 1);
        assert_eq!(grid.group_at(1, 1).unwrap().volume(), 1);
        assert_eq!(grid.group_at(1, 2).unwrap().volume(), 1);

        assert_eq!(grid.group_at(2, 0).unwrap().volume(), 1);
        assert_eq!(grid.group_at(2, 1).unwrap().volume(), 1);
        assert_eq!(grid.group_at(2, 2).unwrap().volume(), 1);
    }

    #[test]
    fn test_polars_data_1x3x1() {
        /**
         property_1     1   2   3    - X
         property_2
            1           11  12  13

            Y

        **/
        let mut df = df!(
            "property_1" => &[1,   2,   3,  ],
            "property_2" => &[10,  10,  10, ],
            "property_3" => &[11,  12,  13, ]
        )
        .unwrap();

        debug!("wapuku: df: {:?}", df);

        // let mut grid = x_property_1_y_property_2_to_3_x_3_data(df, (Some(1_i64), Some(4_i64)), (Some(10_i64), Some(10_i64)));
        let mut grid = x_property_1_y_property_2_to_3_x_3_data(df, (None, None), (None, None));

        debug!("wapuku: grid: {:?}", grid);

        // let data = grid.data();

        // assert_eq!(grid.group_at(0, 0).is_none(), true);
        // assert_eq!(grid.group_at(0, 1).is_none(), true);
        // assert_eq!(grid.group_at(0, 2).is_none(), true);

        assert_eq!(grid.group_at(1, 0).unwrap().volume(), 1);
        assert_eq!(grid.group_at(1, 1).unwrap().volume(), 1);
        assert_eq!(grid.group_at(1, 2).unwrap().volume(), 1);

        // assert_eq!(grid.group_at(2, 0).is_none(), true);
        // assert_eq!(grid.group_at(2, 1).is_none(), true);
        // assert_eq!(grid.group_at(2, 2).is_none(), true);
    }

    #[test]
    fn test_polars_data_2x3x3() {
        /**
         property_1     1   2   3    - X
         property_2
            1           11  12  13
            2           21  22  23
            3           31  32  33

            Y

        **/

        let mut df = df!(
            "property_1" => &[1,   2,   3,   1,   2,   3,   1,   2,   3,    1,   2,   3,   1,   2,   3,   1,   2,   3, ],
            "property_2" => &[10,  10,  10,  20,  20,  20,  30,  30,  30,   10,  10,  10,  20,  20,  20,  30,  30,  30,],
            "property_3" => &[11,  12,  13,  21,  22,  23,  31,  32,  33,   110, 120, 130, 210, 220, 230, 310, 320, 330,]
        ).unwrap();

        debug!("wapuku: df: {:?}", df);

        // let mut grid = x_property_1_y_property_2_to_3_x_3_data(df, (Some(1i64), Some(4i64)), (Some(10i64), Some(31i64)));
        let grid = x_property_1_y_property_2_to_3_x_3_data(df, (None, None), (None, None));

        debug!("wapuku: grid: {:?}", grid);

        // let data = grid.data();

        assert_eq!(grid.group_at(0, 0).unwrap().volume(), 2);
        assert_eq!(grid.group_at(0, 1).unwrap().volume(), 2);
        assert_eq!(grid.group_at(0, 2).unwrap().volume(), 2);

        assert_eq!(grid.group_at(1, 0).unwrap().volume(), 2);
        assert_eq!(grid.group_at(1, 1).unwrap().volume(), 2);
        assert_eq!(grid.group_at(1, 2).unwrap().volume(), 2);

        assert_eq!(grid.group_at(2, 0).unwrap().volume(), 2);
        assert_eq!(grid.group_at(2, 1).unwrap().volume(), 2);
        assert_eq!(grid.group_at(2, 2).unwrap().volume(), 2);
    }

    #[test]
    fn test_polars_data_1x2x3() {
        /**
         property_1      2   3    - X
         property_2
            1            12  13
            2            22  23
            3            32  33

            Y

        **/
        let mut df = df!(
            "property_1" => &[2,   3,   2,   3,   2,   3,],
            "property_2" => &[10,   20,   30,   10,   20,   30,],
            "property_3" => &[12,  13,  22,  23,  32,  33,]
        )
        .unwrap();

        debug!("wapuku: df: {:?}", df);

        let mut grid = x_property_1_y_property_2_to_3_x_3_data(
            df,
            (Some(1i64), Some(4i64)),
            (Some(10i64), Some(31i64)),
        );
        // let data = grid.data();

        debug!("wapuku: grid={:?}", grid);

        assert_eq!(grid.group_at(0, 0).is_none(), true);
        assert_eq!(grid.group_at(1, 0).unwrap().volume(), 1);
        assert_eq!(grid.group_at(2, 0).unwrap().volume(), 1);

        assert_eq!(grid.group_at(0, 1).is_none(), true);
        assert_eq!(grid.group_at(1, 1).unwrap().volume(), 1);
        assert_eq!(grid.group_at(2, 1).unwrap().volume(), 1);

        assert_eq!(grid.group_at(0, 2).is_none(), true);
        assert_eq!(grid.group_at(1, 2).unwrap().volume(), 1);
        assert_eq!(grid.group_at(2, 2).unwrap().volume(), 1);
    }

    #[test]
    fn test_polars_data_1x3x2() {
        /**
         property_1     1   2   3    - X
         property_2

            2           21  22  23
            3           31  32  33

            Y
        **/
        let mut df = df!(
            "property_1" => &[1,   2,   3,   1,   2,   3,],
            "property_2" => &[20,  20,  20,   30,   30,   30,],
            "property_3" => &[21,  22,  23,  31,  32,  33,]
        )
        .unwrap();

        debug!("wapuku: df: {:?}", df);

        let mut grid = x_property_1_y_property_2_to_3_x_3_data(
            df,
            (Some(1i64), Some(4i64)),
            (Some(10i64), Some(31i64)),
        );
        // let data = gird.data();

        assert_eq!(grid.group_at(0, 0).is_none(), true);
        assert_eq!(grid.group_at(0, 1).unwrap().volume(), 1);
        assert_eq!(grid.group_at(0, 2).unwrap().volume(), 1);

        assert_eq!(grid.group_at(1, 0).is_none(), true);
        assert_eq!(grid.group_at(1, 1).unwrap().volume(), 1);
        assert_eq!(grid.group_at(1, 2).unwrap().volume(), 1);

        assert_eq!(grid.group_at(2, 0).is_none(), true);
        assert_eq!(grid.group_at(2, 1).unwrap().volume(), 1);
        assert_eq!(grid.group_at(2, 2).unwrap().volume(), 1);
    }

    #[test]
    fn test_polars_data_1x4x3() {
        /**
         property_1     1   2   3   4 - X
         property_2
            1           11  12  13  14
            2           21  22  23  24
            3           31  32  33  24

            Y

        **/
        let mut df = df!(
            "property_1" => &[1,   2,   3,   4,   1,   2,   3,   4,  1,   2,   3,  4],
            "property_2" => &[10,  10,  10,  10,  20,  20,  20,  20, 30,  30,  30, 30],
            "property_3" => &[11,  12,  13,  14,  21,  22,  23,  24, 31,  32,  33, 34]
        )
        .unwrap();

        debug!("wapuku: df: {:?}", df);

        let mut grid = x_property_1_y_property_2_to_3_x_3_data(
            df,
            (Some(0i64), Some(5i64)),
            (Some(0i64), Some(31i64)),
        );
        // let data = grid.data();
        assert_eq!(grid.group_at(0, 0).unwrap().volume(), 1);
        assert_eq!(grid.group_at(1, 0).unwrap().volume(), 2);
        assert_eq!(grid.group_at(2, 0).unwrap().volume(), 1);

        assert_eq!(grid.group_at(0, 1).unwrap().volume(), 1);
        assert_eq!(grid.group_at(1, 1).unwrap().volume(), 2);
        assert_eq!(grid.group_at(2, 1).unwrap().volume(), 1);

        assert_eq!(grid.group_at(0, 2).unwrap().volume(), 1);
        assert_eq!(grid.group_at(1, 2).unwrap().volume(), 2);
        assert_eq!(grid.group_at(2, 2).unwrap().volume(), 1);
    }

    fn x_property_1_y_property_2_to_3_x_3_data(
        mut df: DataFrame,
        min_max_x: (Option<i64>, Option<i64>),
        min_max_y: (Option<i64>, Option<i64>),
    ) -> GroupsGrid {
        let mut data = PolarsData::new(df, String::from("test"));

        let all_properties: HashSet<&dyn Property> = data.all_properties();

        let (property_1, property_2, property_3) = {
            let mut all_properties_iter =
                all_properties.into_iter().collect::<Vec<&dyn Property>>();
            all_properties_iter.sort_by(|p1, p2| p1.name().cmp(p2.name()));

            (
                *all_properties_iter.get(0).expect("property_1"),
                *all_properties_iter.get(1).expect("property_2"),
                *all_properties_iter.get(2).expect("property_3"),
            )
        };

        data.build_grid(
            PropertyRange::new(property_1, min_max_x.0, min_max_x.1),
            PropertyRange::new(property_2, min_max_y.0, min_max_y.1),
            3,
            3,
            "property_3",
        )
    }

    #[test]
    #[cfg(perf)]
    fn test_polars_data() {
        let df = df!(
            "property_1" => &(0..10_000_000).into_iter().map(|i|i / 10).collect::<Vec<i64>>(), // 10 X 0, 10 X 1 ...
            "property_2" => &(0..10_000_000).into_iter().map(|i|i - (i/10)*10 ).collect::<Vec<i64>>(), //
            "property_3" => &(0..10_000_000).into_iter().map(|i|i).collect::<Vec<i32>>(),
        )
        .unwrap();

        debug!("wapuku: df: {:?}", df);
        let t_0 = Instant::now();

        let mut data = PolarsData::new(df);

        let all_properties: HashSet<&dyn Property> = data.all_properties();

        let (property_1, property_2, property_3) = {
            let mut all_properties_iter =
                all_properties.into_iter().collect::<Vec<&dyn Property>>();
            all_properties_iter.sort_by(|p1, p2| p1.name().cmp(p2.name()));

            (
                *all_properties_iter.get(0).expect("property_1"),
                *all_properties_iter.get(1).expect("property_2"),
                *all_properties_iter.get(2).expect("property_3"),
            )
        };

        let mut data_grid = data.build_grid(
            PropertyRange::new(property_1, None, None),
            PropertyRange::new(property_2, None, None),
            3,
            3,
            "property_3",
        );

        let data = data_grid.data();

        debug!("wapuku: done in {}", t_0.elapsed().as_millis());
    }

    #[test]
    fn test_build_grid_with_more_properties() {
        let mut df = df!(
            "property_1" => &[10,      20,     30,     40,    50,      60,      70,      80,      90, 100],
            "property_2" => &[1,       1,      1,      1,     2,       2,       2,       2,       2,    2],
            "property_3" => &["a",     "b",    "c",    "d",   "e",     "f",     "g",     "h",     "i",  "j"],
            "property_4" => &[100,     200,    300,    400,   500,     600,     700,     800,     900,  1000],
            "property_5" => &[1000,    2000,   3000,   4000,  5000,    6000,    7000,    8000,    9000, 10000]
        ).unwrap();

        debug!("wapuku: {:?}", df);

        let mut polars_data = PolarsData::new(df, String::from("test"));

        let all_sets = polars_data.all_sets();

        debug!("wapuku: all_sets={:?}", all_sets);
        assert_eq!(all_sets.len(), 1);

        let all_properties = polars_data.all_properties();
        assert_eq!(all_properties.len(), 5);

        debug!("all_properties={:?}", all_properties);

        let (property_1, property_2, property_3) = {
            (
                *all_properties
                    .iter()
                    .find(|p| p.name().eq("property_1"))
                    .expect("property_1"),
                *all_properties
                    .iter()
                    .find(|p| p.name().eq("property_2"))
                    .expect("property_2"),
                *all_properties
                    .iter()
                    .find(|p| p.name().eq("property_3"))
                    .expect("property_3"),
            )
        };

        let mut data_grid = polars_data.build_grid(
            PropertyRange::new(property_1, None, None),
            PropertyRange::new(property_2, None, None),
            3,
            3,
            "property_3",
        );
        //TODO
        debug!("wapuku: data_grid={:?}", data_grid.data());
    }

    #[test]
    fn test_group_by_same_order() {
        let mut df = df!(
            "field_1" => &[10,      20,     30,     40,   41,    50,     60,     70,     80,     90],
            "field_2" => &[1,       1,      1,      1,    3,     2,      2,      2,      2,      2],
            "field_3" => &["a",     "b",    "c",    "d",  "dd",  "e",   "f",    "g",    "h",    "ii"]
        ).unwrap();

        /*
        _____|_0-20_|_20-40_|_40-60_|_60-80_|_80-100_|
         0-2 |  a      bc      d
         2-4 |                dd e    fg      h ii
         4-6 |
         6-8 |
         8-10|

         */
        let df = group_by_2(
            &df,
            "field_2",
            2,
            "field_1",
            20,
            [col("field_3").alias("field_3_value")],
            0i64,
            0i64,
        )
        .expect("df");

        debug!("3. wapuku: df={:?}", df);

        assert_eq!(
            *df.column("field_3_value").expect("field_3_value"),
            Series::new(
                "field_3_value",
                [
                    List(Series::new("", ["a"])),
                    List(Series::new("", ["b", "c"])),
                    List(Series::new("", ["d"])),
                    List(Series::new("", ["h", "ii"])),
                    List(Series::new("", ["dd", "e"])),
                    List(Series::new("", ["f", "g"])),
                ]
            )
        );
    }

    #[test]
    fn test_corr() {
        let df = df!(
            "property_1" => &[10,      20,     30,     40,    50,      60,      70,      80,      90,   100],
            "property_2" => &[1,       2,      3,      4,     5,       6,       7,       8,       9,    10],
            "property_3" => &["a",     "a",    "b",    "b",   "c",     "c",     "d",     "d",     "e",  "e"],
            "property_4" => &[10,      9,      8,      7,     6,       5,       4,       3,       2,    1],
            "property_5" => &[1000,    2000,   3000,   4000,  5000,    6000,    7000,    8000,    9000, 10000]
        ).unwrap();

        debug!("wapuku: {:?}", df);

        let polars_data = PolarsData::new(df, String::from("test"));

        let res = polars_data.clc_corrs(
            0,
            vec![
                "property_1".into(),
                "property_2".into(),
                "property_3".into(),
                "property_4".into(),
            ],
        );

        let corrs = res.expect("corrs");

        assert_eq!(
            corrs
                .values()
                .get(&("property_1".into(), "property_2".into()))
                .expect("property_1 property_2"),
            &1.
        );
        assert_eq!(
            corrs
                .values()
                .get(&("property_1".into(), "property_4".into()))
                .expect("property_1 property_2"),
            &-1.
        );
        assert_eq!(
            corrs
                .values()
                .get(&("property_1".into(), "property_3".into()))
                .expect("property_1 property_3"),
            &0.9847319
        );
    }

    #[test]
    fn test_simp() {
        let df = df!(
            // "property_1" => &(0..10_000_000).into_iter().map(|i|i / 10).collect::<Vec<i64>>(), // 10 X 0, 10 X 1 ...
            // "property_2" => &(0..10_000_000).into_iter().map(|i|i - (i/10)*10 ).collect::<Vec<i64>>(), //
            // "property_3" => &(0..10_000_000).into_iter().map(|i|i).collect::<Vec<i32>>(),

            // "property_1" => &[1,   2,   3,   1,   2,   3,   1,   2,   3,],
            // "property_2" => &[10,  10,  10,  20,  20,  20,  30,  30,  30,],
            // "property_3" => &[11,  12,  13,  21,  22,  23,  31,  32,  33,]

            "property_1" => &(0..10000).into_iter().map(|i|i / 100).collect::<Vec<i64>>(), // 10 X 0, 10 X 1 ...
            "property_2" => &(0..10000).into_iter().map(|i|i - (i/100)*100 ).collect::<Vec<i64>>(), //
            "property_3" => &(0..10000).into_iter().map(|i|i).collect::<Vec<i32>>(),
        )
        .unwrap();

        println!(
            "df={:?}",
            PolarsData::new(df, String::from("test")).build_grid(
                PropertyRange::new(
                    &DataProperty::new(WapukuDataType::Numeric, "property_1"),
                    Some(-1),
                    Some(100)
                ),
                PropertyRange::new(
                    &DataProperty::new(WapukuDataType::Numeric, "property_2"),
                    Some(-1),
                    Some(100)
                ),
                3,
                3,
                "property_3",
            )
        );

        /* // let group_by = df.groupby(["field_1"]).unwrap();
        // let by = group_by.select(["field_2"]);
        // let df = by.groups().unwrap();

        let df = df.clone().lazy().groupby_dynamic(
            [],
            DynamicGroupOptions {
                index_column: "field_1".into(),
                every: Duration::new(2),
                period: Duration::new(2),
                offset: Duration::new(-1),
                include_boundaries: true,
                closed_window: ClosedWindow::Left,
                start_by: StartBy::WindowBound,
            }

        )
            .agg([col("field_2").alias("field_2"), col("field_3")])
            .collect().unwrap();

        debug!("wapuku: parquet_scan: df={:?}", df);*/
    }

    pub fn fake_df() -> DataFrame {
        // df!(
        //     "property_1" => &(0..10_000_000).into_iter().map(|i|i / 10).collect::<Vec<i64>>(), // 10 X 0, 10 X 1 ...
        //     "property_2" => &(0..10_000_000).into_iter().map(|i|i - (i/10)*10 ).collect::<Vec<i64>>(), //
        //     "property_3" => &(0..10_000_000).into_iter().map(|i|i).collect::<Vec<i32>>(),
        // ).unwrap()
        // df!(
        //     "property_1" => &[1,   2,   3,   1,   2,   3,   1,   2,   3,],
        //     "property_2" => &[10,  10,  10,  20,  20,  20,  30,  30,  30,],
        //     "property_3" => &[11,  12,  13,  21,  22,  23,  31,  32,  33,]
        // ).unwrap()

        df!(
           "property_1" => &(0..10_000).into_iter().map(|i|i / 100).collect::<Vec<i64>>(), // 10 X 0, 10 X 1 ...
           "property_2" => &(0..10_000).into_iter().map(|i|i - (i/100)*100 ).collect::<Vec<i64>>(), //
           "property_3" => &(0..10_000).into_iter().map(|i|i).collect::<Vec<i32>>(),
        )
        .unwrap()
    }

    #[test]
    fn test_describe() -> PolarsResult<()> {
        std::env::set_var("POLARS_FMT_MAX_COLS", "100"); //FMT_MAX_COLS is pub(crate) in polars_core :(
        std::env::set_var("POLARS_FMT_MAX_ROWS", "100");

        let df = all_types_df();

        let summary_df = describe(&df.lazy())?;

        let desc_cl = summary_df
            .column("describe")?
            .str()?
            .into_iter()
            .map(|v| v.unwrap_or_default())
            .collect::<Vec<&str>>();

        let float_cl = summary_df
            .column("float")?
            .f64()?
            .into_iter()
            .map(|v| format!("{:.5}", v.unwrap_or_default()))
            .collect::<Vec<String>>();

        let int_cl = summary_df
            .column("int")?
            .f64()?
            .into_iter()
            .map(|v| format!("{:.5}", v.unwrap_or_default()))
            .collect::<Vec<String>>();
        let bool_cl = summary_df
            .column("bool")?
            .str()?
            .into_iter()
            .map(|v| v.unwrap_or_default())
            .collect::<Vec<&str>>();
        let str_cl = summary_df
            .column("str")?
            .str()?
            .into_iter()
            .map(|v| v.unwrap_or_default())
            .collect::<Vec<&str>>();
        let str2_cl = summary_df
            .column("str2")?
            .str()?
            .into_iter()
            .map(|v| v.unwrap_or_default())
            .collect::<Vec<&str>>();
        let date_cl = summary_df
            .column("date")?
            .str()?
            .into_iter()
            .map(|v| v.unwrap_or_default())
            .collect::<Vec<&str>>();

        /*      from py-polars/polars/dataframe/frame.py:4394
               ┌────────────┬──────────┬──────────┬───────┬──────┬──────┬────────────┐
               │ describe   ┆ float    ┆ int      ┆ bool  ┆ str  ┆ str2 ┆ date      │
               │ ---        ┆ ---      ┆ ---      ┆ ---   ┆ ---  ┆ ---  ┆ ---       │
               │ str        ┆ f64      ┆ f64      ┆ str   ┆ str  ┆ str  ┆ str       │
               ╞════════════╪══════════╪══════════╪═══════╪══════╪══════╪════════════╡
               │ count      ┆ 3.0      ┆ 2.0      ┆ 3     ┆ 2    ┆ 2    ┆ 3         │
               │ null_count ┆ 0.0      ┆ 1.0      ┆ 0     ┆ 1    ┆ 1    ┆ 0         │
               │ mean       ┆ 2.266667 ┆ 4.5      ┆ null  ┆ null ┆ null ┆ null      │
               │ std        ┆ 1.101514 ┆ 0.707107 ┆ null  ┆ null ┆ null ┆ null      │
               │ min        ┆ 1.0      ┆ 4.0      ┆ False ┆ b    ┆ eur  ┆ 2020-01-01│
               │ 25%        ┆ 2.8      ┆ 4.0      ┆ null  ┆ null ┆ null ┆ null      │
               │ 50%        ┆ 2.8      ┆ 5.0      ┆ null  ┆ null ┆ null ┆ null      │
               │ 75%        ┆ 3.0      ┆ 5.0      ┆ null  ┆ null ┆ null ┆ null      │
               │ max        ┆ 3.0      ┆ 5.0      ┆ True  ┆ c    ┆ usd  ┆ 2022-01-01│
               └────────────┴──────────┴──────────┴───────┴──────┴──────┴────────────┘
        */

        assert_eq!(desc_cl[0], "count");
        assert_eq!(desc_cl[1], "null_count");

        assert_eq!(float_cl[0], "3.00000");
        assert_eq!(int_cl[0], "2.00000");
        assert_eq!(bool_cl[0], "3");
        assert_eq!(str_cl[0], "2");
        assert_eq!(str2_cl[0], "2");
        assert_eq!(date_cl[0], "3");

        assert_eq!(float_cl[1], "0.00000");
        assert_eq!(int_cl[1], "1.00000");
        assert_eq!(bool_cl[1], "0");

        assert_eq!(desc_cl[2], "mean");
        assert_eq!(float_cl[2], "2.26667");
        assert_eq!(int_cl[2], "4.50000");
        assert_eq!(bool_cl[2], "");

        assert_eq!(desc_cl[3], "std");
        assert_eq!(float_cl[3], "1.10151");
        assert_eq!(int_cl[3], "0.70711");
        assert_eq!(bool_cl[3], "");

        assert_eq!(desc_cl[4], "min");
        assert_eq!(float_cl[4], "1.00000");
        assert_eq!(int_cl[4], "4.00000");
        assert_eq!(bool_cl[4], "false");

        assert_eq!(desc_cl[5], "25%");
        assert_eq!(float_cl[5], "2.80000");
        assert_eq!(int_cl[5], "4.00000");
        assert_eq!(bool_cl[5], "");
        assert_eq!(desc_cl[6], "50%");

        assert_eq!(float_cl[6], "2.80000");
        assert_eq!(int_cl[6], "5.00000");
        assert_eq!(bool_cl[6], "");

        assert_eq!(desc_cl[7], "75%");
        assert_eq!(float_cl[7], "3.00000");
        assert_eq!(int_cl[7], "5.00000");
        assert_eq!(bool_cl[7], "");

        assert_eq!(desc_cl[8], "max");
        assert_eq!(float_cl[8], "3.00000");
        assert_eq!(int_cl[8], "5.00000");
        assert_eq!(bool_cl[8], "true");

        assert_eq!(str_cl[1], "1");
        assert_eq!(str2_cl[1], "1");
        assert_eq!(date_cl[1], "0");
        assert_eq!(str_cl[2], "");
        assert_eq!(str2_cl[2], "");
        assert_eq!(date_cl[2], "");
        assert_eq!(str_cl[3], "");
        assert_eq!(str2_cl[3], "");
        assert_eq!(date_cl[3], "");
        assert_eq!(str_cl[4], "b");
        assert_eq!(str2_cl[4], "eur");
        assert_eq!(date_cl[4], "2020-01-01");
        assert_eq!(str_cl[5], "");
        assert_eq!(str2_cl[5], "");
        assert_eq!(date_cl[5], "");
        assert_eq!(str_cl[6], "");
        assert_eq!(str2_cl[6], "");
        assert_eq!(date_cl[6], "");
        assert_eq!(str_cl[7], "");
        assert_eq!(str2_cl[7], "");
        assert_eq!(date_cl[7], "");
        assert_eq!(str_cl[8], "c");
        assert_eq!(str2_cl[8], "usd");
        assert_eq!(date_cl[8], "2022-01-01");

        Ok(())
    }

    #[test]
    fn test_describe_with_extra_aggs() -> PolarsResult<()> {
        std::env::set_var("POLARS_FMT_MAX_COLS", "100");
        std::env::set_var("POLARS_FMT_MAX_ROWS", "100");

        let df = all_types_df();

        let summary_df = describe_with_params(
            &df.lazy(),
            false,
            vec![(
                "kurtosis".to_owned(),
                Box::new(|dt: &DataType| dt.is_numeric()) as Box<dyn Fn(&DataType) -> bool>,
                Box::new(move |name: &String| {
                    col(name)
                        .kurtosis(true, true)
                        .alias(format!("{} kurtosis", name).as_str())
                }) as Box<dyn Fn(&String) -> Expr>,
            )],
            Some(vec!["float"]),
        )?;

        assert_eq!(
            summary_df
                .get_columns()
                .iter()
                .map(|s| s.name())
                .collect::<Vec<&str>>(),
            vec!["name", "kurtosis"]
        );

        assert_eq!(
            summary_df
                .column("name")?
                .str()?
                .into_iter()
                .collect::<Vec<Option<&str>>>()[0],
            Some("float"),
        );

        let kurtosis_cl = summary_df
            .column("kurtosis")?
            .f64()?
            .into_iter()
            .map(|v| format!("{:.5}", v.unwrap_or_default()))
            .collect::<Vec<String>>();

        assert_eq!(kurtosis_cl, ["-1.50000"]);

        Ok(())
    }

    #[test]
    fn test_describe_nan() -> PolarsResult<()> {
        std::env::set_var("POLARS_FMT_MAX_COLS", "100");
        std::env::set_var("POLARS_FMT_MAX_ROWS", "100");

        let df = DataFrame::new(vec![Series::new("days", [f32::NAN].as_ref())])?;

        let summary_df = describe(&df.lazy())?;

        assert_eq!(
            summary_df
                .column("days")?
                .f64()?
                .into_iter()
                .collect::<Vec<Option<f64>>>()[0..2],
            vec![
                Some(1.0),
                Some(0.0),
                Some(f64::NAN),
                None,
                Some(f64::NAN),
                Some(f64::NAN),
                Some(f64::NAN),
                Some(f64::NAN),
                Some(f64::NAN)
            ][0..2]
        );

        Ok(())
    }

    fn all_types_df() -> DataFrame {
        df![
        "float" => [Some(1.0), Some(2.8), Some(3.0)],
        "int" =>  [Some(4), Some(5), None],
        "bool" =>  [true, false, true],
        "str" =>  [None, Some("b"), Some("c")],
        "str2" =>  [Some("usd"), Some("eur"), None],
        "date" =>  [NaiveDate::from_ymd_opt(2020, 1, 1).unwrap(), NaiveDate::from_ymd_opt(2021, 1, 1).unwrap(), NaiveDate::from_ymd_opt(2022, 1, 1).unwrap()],
    ].unwrap()
    }
}
