use crate::chunk::{DBConnectionMySQL, DBStatementMySQL,
                   DBConnectionPostgres, DBStatementPostgres,
                   DBConnectionSQLite, DBStatementSQLite,
                   Value};
use crate::vm::*;
use chrono::Utc;
use futures::executor::block_on;
use num_bigint::BigInt;
use num_traits::FromPrimitive;
use sqlx::{Column, Row, TypeInfo};

impl VM {
    /// Takes a database type, hostname, database name, username, and
    /// password as its arguments, and returns a database connection
    /// for the specified database.
    /// (The database handling here would be simpler if Any were used,
    /// but its type support for the different database engines is a bit
    /// patchy.)
    pub fn core_db_conn(&mut self) -> i32 {
        block_on(async {
            if self.stack.len() < 2 {
                self.print_error("db.conn requires at least two arguments");
                return 0;
            }

            let dbtype = self.stack.pop().unwrap();
            let host = self.stack.pop().unwrap();

            let dbtype_str_opt: Option<&str>;
            to_str!(dbtype, dbtype_str_opt);
            let host_str_opt: Option<&str>;
            to_str!(host, host_str_opt);

            match (dbtype_str_opt, host_str_opt) {
                (Some(dbtype_str), Some(host_str)) => {
                    match dbtype_str {
                        "mysql" => {
                            if self.stack.len() < 3 {
                                self.print_error("db.conn requires five arguments for MySQL");
                                return 0;
                            }
                            let db = self.stack.pop().unwrap();
                            let pass = self.stack.pop().unwrap();
                            let user = self.stack.pop().unwrap();
                            let db_str_opt: Option<&str>;
                            to_str!(db, db_str_opt);
                            let pass_str_opt: Option<&str>;
                            to_str!(pass, pass_str_opt);
                            let user_str_opt: Option<&str>;
                            to_str!(user, user_str_opt);
                            match (db_str_opt, user_str_opt, pass_str_opt) {
                                (Some(db_str), Some(user_str), Some(pass_str)) => {
                                    let url = format!("{}://{}:{}@{}/{}",
                                        dbtype_str, user_str, pass_str, host_str, db_str);
                                    let pool_res = sqlx::Pool::connect(&url).await;
                                    match pool_res {
                                        Ok(pool) => {
                                            let dbc = DBConnectionMySQL::new(pool);
                                            let dbcv = Value::DBConnectionMySQL(Rc::new(RefCell::new(dbc)));
                                            self.stack.push(dbcv);
                                            return 1;
                                        }
                                        Err(e) => {
                                            let err_str = format!("unable to connect to database: {}", e);
                                            self.print_error(&err_str);
                                            return 0;
                                        }
                                    }
                                }
                                _ => {
                                    self.print_error("db.conn arguments must be strings");
                                    return 0;
                                }
                            }
                        }
                        "postgresql" => {
                            if self.stack.len() < 3 {
                                self.print_error("db.conn requires five arguments for PostgreSQL");
                                return 0;
                            }
                            let db = self.stack.pop().unwrap();
                            let pass = self.stack.pop().unwrap();
                            let user = self.stack.pop().unwrap();
                            let db_str_opt: Option<&str>;
                            to_str!(db, db_str_opt);
                            let pass_str_opt: Option<&str>;
                            to_str!(pass, pass_str_opt);
                            let user_str_opt: Option<&str>;
                            to_str!(user, user_str_opt);
                            match (db_str_opt, user_str_opt, pass_str_opt) {
                                (Some(db_str), Some(user_str), Some(pass_str)) => {
                                    let url = format!("{}://{}:{}@{}/{}",
                                        dbtype_str, user_str, pass_str, host_str, db_str);
                                    let pool_res = sqlx::Pool::connect(&url).await;
                                    match pool_res {
                                        Ok(pool) => {
                                            let dbc = DBConnectionPostgres::new(pool);
                                            let dbcv = Value::DBConnectionPostgres(Rc::new(RefCell::new(dbc)));
                                            self.stack.push(dbcv);
                                            return 1;
                                        }
                                        Err(e) => {
                                            let err_str = format!("unable to connect to database: {}", e);
                                            self.print_error(&err_str);
                                            return 0;
                                        }
                                    }
                                }
                                _ => {
                                    self.print_error("db.conn arguments must be strings");
                                    return 0;
                                }
                            }
                        }
                        "sqlite" => {
                            let url = format!("{}://{}", dbtype_str, host_str);
                            let pool_res = sqlx::Pool::connect(&url).await;
                            match pool_res {
                                Ok(pool) => {
                                    let dbc = DBConnectionSQLite::new(pool);
                                    let dbcv = Value::DBConnectionSQLite(Rc::new(RefCell::new(dbc)));
                                    self.stack.push(dbcv);
                                    return 1;
                                }
                                Err(e) => {
                                    let err_str = format!("unable to connect to database: {}", e);
                                    self.print_error(&err_str);
                                    return 0;
                                }
                            }
                        }
                        _ => {
                            self.print_error("invalid database type");
                            return 0;
                        }
                    }
                }
                _ => {
                    self.print_error("db.conn arguments must be strings");
                    return 0;
                }
            }
        })
    }

    /// Takes an SQL string and a database connection as its
    /// arguments.  Returns a statement object that can be used to
    /// execute the query and fetch the associated results.
    /// (For now, this does not actually prepare the query, so as to
    /// avoid lifetime issues.  It may be updated later to support
    /// that.)
    pub fn core_db_prep(&mut self) -> i32 {
        if self.stack.len() < 2 {
            self.print_error("db.prep requires two arguments");
            return 0;
        }

        let sql = self.stack.pop().unwrap();
        let dbcv = self.stack.pop().unwrap();

        let sql_str_opt: Option<&str>;
        to_str!(sql, sql_str_opt);

        match (dbcv, sql_str_opt) {
            (Value::DBConnectionMySQL(dbc), Some(sql_str)) => {
                let dbs = DBStatementMySQL::new(dbc.borrow().pool.clone(), sql_str.to_string());
                let dbsv = Value::DBStatementMySQL(Rc::new(RefCell::new(dbs)));
                self.stack.push(dbsv);
                return 1;
            }
            (Value::DBConnectionPostgres(dbc), Some(sql_str)) => {
                let dbs = DBStatementPostgres::new(dbc.borrow().pool.clone(), sql_str.to_string());
                let dbsv = Value::DBStatementPostgres(Rc::new(RefCell::new(dbs)));
                self.stack.push(dbsv);
                return 1;
            }
            (Value::DBConnectionSQLite(dbc), Some(sql_str)) => {
                let dbs = DBStatementSQLite::new(dbc.borrow().pool.clone(), sql_str.to_string());
                let dbsv = Value::DBStatementSQLite(Rc::new(RefCell::new(dbs)));
                self.stack.push(dbsv);
                return 1;
            }
            (Value::DBConnectionMySQL(_), _) => {
                self.print_error("second db.prep argument must be string");
                return 0;
            }
            (Value::DBConnectionPostgres(_), _) => {
                self.print_error("second db.prep argument must be string");
                return 0;
            }
            (Value::DBConnectionSQLite(_), _) => {
                self.print_error("second db.prep argument must be string");
                return 0;
            }
            _ => {
                self.print_error("first db.prep argument must be database connection");
                return 0;
            }
        }
    }

    fn db_exec_mysql(&mut self, dbsv: &mut Rc<RefCell<DBStatementMySQL>>,
                     lst: Rc<RefCell<VecDeque<Value>>>) -> i32 {
	block_on(async {
	    let mut dbsvb = dbsv.borrow_mut();
	    let query = dbsvb.query.clone();
	    let pool = &mut dbsvb.pool;
	    let mut conn = pool.acquire().await.unwrap();
	    let mut query_obj = sqlx::query(&query);
	    let lstb = lst.borrow();
	    for param in lstb.iter() {
		match param {
		    Value::String(s) => {
			query_obj =
			    query_obj.bind(s.borrow().string.clone());
			continue;
		    }
		    Value::Int(n) => {
			query_obj = query_obj.bind(n);
			continue;
		    }
		    _ => {}
		}
		let param_str_opt: Option<&str>;
		to_str!(param, param_str_opt);
		match param_str_opt {
		    Some(s) => {
			query_obj = query_obj.bind(s.to_string());
		    }
		    _ => {
			self.print_error("unable to process db.exec parameter");
			return 0;
		    }
		}
	    }
	    let query_res = query_obj.fetch_all(&mut conn).await;
	    let mut records = VecDeque::new();
	    let raw_records;
	    match query_res {
		Ok(res) => {
		    raw_records = res;
		}
		Err(e) => {
		    let err_str = format!("unable to execute query: {}", e);
		    self.print_error(&err_str);
		    return 0;
		}
	    }
	    for raw_record in raw_records {
		let mut ret_record = IndexMap::new();
		for column in raw_record.columns() {
		    let name = column.name();
		    let index = column.ordinal();
		    let type_info = column.type_info();
		    if type_info.is_null() {
			ret_record.insert(
			    name.to_string(),
			    Value::Null
			);
			continue;
		    }
		    match type_info.name() {
			"BOOLEAN" => {
			    let final_value_res =
				raw_record.get::<Option<bool>, usize>(index);
			    match final_value_res {
				None => {
				    ret_record.insert(
					name.to_string(),
					Value::Null
				    );
				}
				Some(b) => {
				    ret_record.insert(
					name.to_string(),
					Value::Bool(b)
				    );
				}
			    }
			}
			"VARCHAR" | "CHAR" | "TINYTEXT" | "TEXT" | "MEDIUMTEXT" | "LONGTEXT" | "ENUM" => {
			    let final_value_res =
				raw_record.get::<Option<String>, usize>(index);
			    match final_value_res {
				None => {
				    ret_record.insert(
					name.to_string(),
					Value::Null
				    );
				}
				Some(s) => {
				    ret_record.insert(
					name.to_string(),
                                        new_string_value(s.to_string())
				    );
				}
			    }
			}
			"BINARY" | "VARBINARY" | "TINYBLOB" | "BLOB" | "MEDIUMBLOB" | "LONGBLOB" => {
			    let bytes_res =
				raw_record.get::<Option<Vec<u8>>, usize>(index);
			    match bytes_res {
				None => {
				    ret_record.insert(
					name.to_string(),
					Value::Null
				    );
				}
				Some(bytes) => {
				    let mut lst = VecDeque::new();
				    for i in bytes {
					lst.push_back(Value::Byte(i));
				    }
				    ret_record.insert(
					name.to_string(),
					Value::List(Rc::new(RefCell::new(lst)))
				    );
				}
			    }
			}
			"DATE" => {
			    let final_value_res =
				raw_record.get::<Option<chrono::NaiveDate>, usize>(index);
			    match final_value_res {
				None => {
				    ret_record.insert(
					name.to_string(),
					Value::Null
				    );
				}
				Some(final_value) => {
				    ret_record.insert(
					name.to_string(),
                                        new_string_value(final_value.to_string())
				    );
				}
			    }
			}
			"TIME" => {
			    let final_value_res =
				raw_record.get::<Option<chrono::NaiveTime>, usize>(index);
			    match final_value_res {
				None => {
				    ret_record.insert(
					name.to_string(),
					Value::Null
				    );
				}
				Some(final_value) => {
				    ret_record.insert(
					name.to_string(),
                                        new_string_value(final_value.to_string())
				    );
				}
			    }
			}
			"DATETIME" => {
			    let dt_res =
				raw_record.get::<Option<chrono::NaiveDateTime>, usize>(index);
			    match dt_res {
				None => {
				    ret_record.insert(
					name.to_string(),
					Value::Null
				    );
				}
				Some(dt) => {
				    let final_value = dt.and_local_timezone(self.utc_tz).unwrap();
				    ret_record.insert(
					name.to_string(),
					Value::DateTimeNT(final_value)
				    );
				}
			    }
			}
			"TIMESTAMP" => {
			    let dt_res =
				raw_record.get::<Option<chrono::DateTime<Utc>>, usize>(index);
			    match dt_res {
				None => {
				    ret_record.insert(
					name.to_string(),
					Value::Null
				    );
				}
				Some(dt) => {
				    let final_value = dt.with_timezone(&self.utc_tz);
				    ret_record.insert(
					name.to_string(),
					Value::DateTimeNT(final_value)
				    );
				}
			    }
			}
			"TINYINT" | "SMALLINT" | "MEDIUMINT" | "INT" => {
			    let final_value_res =
				raw_record.get::<Option<i32>, usize>(index);
			    match final_value_res {
				None => {
				    ret_record.insert(
					name.to_string(),
					Value::Null
				    );
				}
				Some(final_value) => {
				    ret_record.insert(
					name.to_string(),
					Value::Int(final_value)
				    );
				}
			    }
			}
			"BIGINT" => {
			    let final_value_res =
				raw_record.get::<Option<i64>, usize>(index);
			    match final_value_res {
				None => {
				    ret_record.insert(
					name.to_string(),
					Value::Null
				    );
				}
				Some(final_value) => {
				    ret_record.insert(
					name.to_string(),
					Value::BigInt(BigInt::from_i64(final_value).unwrap())
				    );
				}
			    }
			}
			"TINYINT UNSIGNED" | "SMALLINT UNSIGNED" | "MEDIUMINT UNSIGNED" => {
			    let final_value_res =
				raw_record.get::<Option<i32>, usize>(index);
			    match final_value_res {
				None => {
				    ret_record.insert(
					name.to_string(),
					Value::Null
				    );
				}
				Some(final_value) => {
				    ret_record.insert(
					name.to_string(),
					Value::Int(final_value)
				    );
				}
			    }
			}
			"INT UNSIGNED" => {
			    let final_value_res =
				raw_record.get::<Option<u32>, usize>(index);
			    match final_value_res {
				None => {
				    ret_record.insert(
					name.to_string(),
					Value::Null
				    );
				}
				Some(final_value) => {
				    ret_record.insert(
					name.to_string(),
					Value::BigInt(BigInt::from_u32(final_value).unwrap())
				    );
				}
			    }
			}
			"BIGINT UNSIGNED" => {
			    let final_value_res =
				raw_record.get::<Option<u64>, usize>(index);
			    match final_value_res {
				None => {
				    ret_record.insert(
					name.to_string(),
					Value::Null
				    );
				}
				Some(final_value) => {
				    ret_record.insert(
					name.to_string(),
					Value::BigInt(BigInt::from_u64(final_value).unwrap())
				    );
				}
			    }
			}
			"FLOAT" => {
			    let final_value_res =
				raw_record.get::<Option<f32>, usize>(index);
			    match final_value_res {
				None => {
				    ret_record.insert(
					name.to_string(),
					Value::Null
				    );
				}
				Some(final_value) => {
				    ret_record.insert(
					name.to_string(),
					Value::Float(final_value as f64)
				    );
				}
			    }
			}
			"DOUBLE" => {
			    let final_value_res =
				raw_record.get::<Option<f64>, usize>(index);
			    match final_value_res {
				None => {
				    ret_record.insert(
					name.to_string(),
					Value::Null
				    );
				}
				Some(final_value) => {
				    ret_record.insert(
					name.to_string(),
					Value::Float(final_value as f64)
				    );
				}
			    }
			}
			"DECIMAL" => {
			    let final_value_res =
				raw_record.get::<Option<rust_decimal::Decimal>, usize>(index);
			    match final_value_res {
				None => {
				    ret_record.insert(
					name.to_string(),
					Value::Null
				    );
				}
				Some(final_value) => {
				    ret_record.insert(
					name.to_string(),
                                        new_string_value(final_value.to_string())
				    );
				}
			    }
			}
			_ => {
			    self.print_error("unable to process database field type");
			    return 0;
			}
		    }
		}
		records.push_back(Value::Hash(Rc::new(RefCell::new(ret_record))));
	    }
	    self.stack.push(Value::List(Rc::new(RefCell::new(records))));
	    return 1;
        })
    }

    fn db_exec_postgres(&mut self, dbsv: &mut Rc<RefCell<DBStatementPostgres>>,
                        lst: Rc<RefCell<VecDeque<Value>>>) -> i32 {
	block_on(async {
	    let mut dbsvb = dbsv.borrow_mut();
	    let query = dbsvb.query.clone();
	    let pool = &mut dbsvb.pool;
	    let mut conn = pool.acquire().await.unwrap();
	    let mut query_obj = sqlx::query(&query);
	    let lstb = lst.borrow();
	    for param in lstb.iter() {
		match param {
		    Value::String(s) => {
			query_obj =
			    query_obj.bind(s.borrow().string.clone());
			continue;
		    }
		    Value::Int(n) => {
			query_obj = query_obj.bind(n);
			continue;
		    }
		    _ => {}
		}
		let param_str_opt: Option<&str>;
		to_str!(param, param_str_opt);
		match param_str_opt {
		    Some(s) => {
			query_obj = query_obj.bind(s.to_string());
		    }
		    _ => {
			self.print_error("unable to process db.exec parameter");
			return 0;
		    }
		}
	    }
	    let query_res = query_obj.fetch_all(&mut conn).await;
	    let mut records = VecDeque::new();
	    let raw_records;
	    match query_res {
		Ok(res) => {
		    raw_records = res;
		}
		Err(e) => {
		    let err_str = format!("unable to execute query: {}", e);
		    self.print_error(&err_str);
		    return 0;
		}
	    }
	    for raw_record in raw_records {
		let mut ret_record = IndexMap::new();
		for column in raw_record.columns() {
		    let name = column.name();
		    let index = column.ordinal();
		    let type_info = column.type_info();
		    if type_info.is_null() {
			ret_record.insert(
			    name.to_string(),
			    Value::Null
			);
			continue;
		    }
		    match type_info.name() {
			"BOOL" => {
			    let final_value_res =
				raw_record.get::<Option<bool>, usize>(index);
			    match final_value_res {
				None => {
				    ret_record.insert(
					name.to_string(),
					Value::Null
				    );
				}
				Some(b) => {
				    ret_record.insert(
					name.to_string(),
					Value::Bool(b)
				    );
				}
			    }
			}
			"CHAR" | "VARCHAR" | "CHAR[]" | "VARCHAR[]" | "TEXT" => {
			    let final_value_res =
				raw_record.get::<Option<String>, usize>(index);
			    match final_value_res {
				None => {
				    ret_record.insert(
					name.to_string(),
					Value::Null
				    );
				}
				Some(s) => {
				    ret_record.insert(
					name.to_string(),
                                        new_string_value(s.to_string())
				    );
				}
			    }
			}
			"BYTEA" | "BYTEA[]" => {
			    let bytes_res =
				raw_record.get::<Option<Vec<u8>>, usize>(index);
			    match bytes_res {
				None => {
				    ret_record.insert(
					name.to_string(),
					Value::Null
				    );
				}
				Some(bytes) => {
				    let mut lst = VecDeque::new();
				    for i in bytes {
					lst.push_back(Value::Byte(i));
				    }
				    ret_record.insert(
					name.to_string(),
					Value::List(Rc::new(RefCell::new(lst)))
				    );
				}
			    }
			}
			"DATE" => {
			    let final_value_res =
				raw_record.get::<Option<chrono::NaiveDate>, usize>(index);
			    match final_value_res {
				None => {
				    ret_record.insert(
					name.to_string(),
					Value::Null
				    );
				}
				Some(final_value) => {
				    ret_record.insert(
					name.to_string(),
                                        new_string_value(final_value.to_string())
				    );
				}
			    }
			}
			"TIME" => {
			    let final_value_res =
				raw_record.get::<Option<chrono::NaiveTime>, usize>(index);
			    match final_value_res {
				None => {
				    ret_record.insert(
					name.to_string(),
					Value::Null
				    );
				}
				Some(final_value) => {
				    ret_record.insert(
					name.to_string(),
                                        new_string_value(final_value.to_string())
				    );
				}
			    }
			}
			"TIMESTAMP" => {
			    let dt_res =
				raw_record.get::<Option<chrono::DateTime<Utc>>, usize>(index);
			    match dt_res {
				None => {
				    ret_record.insert(
					name.to_string(),
					Value::Null
				    );
				}
				Some(dt) => {
				    let final_value = dt.with_timezone(&self.utc_tz);
				    ret_record.insert(
					name.to_string(),
					Value::DateTimeNT(final_value)
				    );
				}
			    }
			}
			"INT2" | "INT4" => {
			    let final_value_res =
				raw_record.get::<Option<i32>, usize>(index);
			    match final_value_res {
				None => {
				    ret_record.insert(
					name.to_string(),
					Value::Null
				    );
				}
				Some(final_value) => {
				    ret_record.insert(
					name.to_string(),
					Value::Int(final_value)
				    );
				}
			    }
			}
			"INT8" => {
			    let final_value_res =
				raw_record.get::<Option<i64>, usize>(index);
			    match final_value_res {
				None => {
				    ret_record.insert(
					name.to_string(),
					Value::Null
				    );
				}
				Some(final_value) => {
				    ret_record.insert(
					name.to_string(),
					Value::BigInt(BigInt::from_i64(final_value).unwrap())
				    );
				}
			    }
			}
			"FLOAT4" => {
			    let final_value_res =
				raw_record.get::<Option<f32>, usize>(index);
			    match final_value_res {
				None => {
				    ret_record.insert(
					name.to_string(),
					Value::Null
				    );
				}
				Some(final_value) => {
				    ret_record.insert(
					name.to_string(),
					Value::Float(final_value as f64)
				    );
				}
			    }
			}
			"FLOAT8" => {
			    let final_value_res =
				raw_record.get::<Option<f64>, usize>(index);
			    match final_value_res {
				None => {
				    ret_record.insert(
					name.to_string(),
					Value::Null
				    );
				}
				Some(final_value) => {
				    ret_record.insert(
					name.to_string(),
					Value::Float(final_value as f64)
				    );
				}
			    }
			}
			_ => {
			    self.print_error("unable to process database field type");
			    return 0;
			}
		    }
		}
		records.push_back(Value::Hash(Rc::new(RefCell::new(ret_record))));
	    }
	    self.stack.push(Value::List(Rc::new(RefCell::new(records))));
	    return 1;
        })
    }

    fn db_exec_sqlite(&mut self, dbsv: &mut Rc<RefCell<DBStatementSQLite>>,
                     lst: Rc<RefCell<VecDeque<Value>>>) -> i32 {
	block_on(async {
	    let mut dbsvb = dbsv.borrow_mut();
	    let query = dbsvb.query.clone();
	    let pool = &mut dbsvb.pool;
	    let mut conn = pool.acquire().await.unwrap();
	    let mut query_obj = sqlx::query(&query);
	    let lstb = lst.borrow();
	    for param in lstb.iter() {
		match param {
		    Value::String(s) => {
			query_obj =
			    query_obj.bind(s.borrow().string.clone());
			continue;
		    }
		    Value::Int(n) => {
			query_obj = query_obj.bind(n);
			continue;
		    }
		    _ => {}
		}
		let param_str_opt: Option<&str>;
		to_str!(param, param_str_opt);
		match param_str_opt {
		    Some(s) => {
			query_obj = query_obj.bind(s.to_string());
		    }
		    _ => {
			self.print_error("unable to process db.exec parameter");
			return 0;
		    }
		}
	    }
	    let query_res = query_obj.fetch_all(&mut conn).await;
	    let mut records = VecDeque::new();
	    let raw_records;
	    match query_res {
		Ok(res) => {
		    raw_records = res;
		}
		Err(e) => {
		    let err_str = format!("unable to execute query: {}", e);
		    self.print_error(&err_str);
		    return 0;
		}
	    }
	    for raw_record in raw_records {
		let mut ret_record = IndexMap::new();
		for column in raw_record.columns() {
		    let name = column.name();
		    let index = column.ordinal();
		    let type_info = column.type_info();
		    if type_info.is_null() {
                        let result =
                            raw_record.try_get_unchecked::<Option<String>, usize>(index);
                        match result {
                            Err(_) => {
                                ret_record.insert(
                                    name.to_string(),
                                    Value::Null
                                );
                            }
                            Ok(final_value) => {
                                ret_record.insert(
                                    name.to_string(),
                                    new_string_value(final_value.unwrap().to_string())
                                );
                            }
                        }
                        continue;
		    }
		    match type_info.name() {
			"BOOLEAN" => {
			    let final_value_res =
				raw_record.get::<Option<bool>, usize>(index);
			    match final_value_res {
				None => {
				    ret_record.insert(
					name.to_string(),
					Value::Null
				    );
				}
				Some(b) => {
				    ret_record.insert(
					name.to_string(),
					Value::Bool(b)
				    );
				}
			    }
			}
			"TEXT" => {
			    let final_value_res =
				raw_record.get::<Option<String>, usize>(index);
			    match final_value_res {
				None => {
				    ret_record.insert(
					name.to_string(),
					Value::Null
				    );
				}
				Some(s) => {
				    ret_record.insert(
					name.to_string(),
                                        new_string_value(s.to_string())
				    );
				}
			    }
			}
			"BLOB" => {
			    let bytes_res =
				raw_record.get::<Option<Vec<u8>>, usize>(index);
			    match bytes_res {
				None => {
				    ret_record.insert(
					name.to_string(),
					Value::Null
				    );
				}
				Some(bytes) => {
				    let mut lst = VecDeque::new();
				    for i in bytes {
					lst.push_back(Value::Byte(i));
				    }
				    ret_record.insert(
					name.to_string(),
					Value::List(Rc::new(RefCell::new(lst)))
				    );
				}
			    }
			}
			"DATE" => {
			    let final_value_res =
				raw_record.get::<Option<chrono::NaiveDate>, usize>(index);
			    match final_value_res {
				None => {
				    ret_record.insert(
					name.to_string(),
					Value::Null
				    );
				}
				Some(final_value) => {
				    ret_record.insert(
					name.to_string(),
                                        new_string_value(final_value.to_string())
				    );
				}
			    }
			}
			"TIME" => {
			    let final_value_res =
				raw_record.get::<Option<chrono::NaiveTime>, usize>(index);
			    match final_value_res {
				None => {
				    ret_record.insert(
					name.to_string(),
					Value::Null
				    );
				}
				Some(final_value) => {
				    ret_record.insert(
					name.to_string(),
                                        new_string_value(final_value.to_string())
				    );
				}
			    }
			}
			"DATETIME" => {
			    let dt_res =
				raw_record.get::<Option<chrono::NaiveDateTime>, usize>(index);
			    match dt_res {
				None => {
				    ret_record.insert(
					name.to_string(),
					Value::Null
				    );
				}
				Some(dt) => {
				    let final_value = dt.and_local_timezone(self.utc_tz).unwrap();
				    ret_record.insert(
					name.to_string(),
					Value::DateTimeNT(final_value)
				    );
				}
			    }
			}
			"INTEGER" => {
			    let final_value_res =
				raw_record.get::<Option<i64>, usize>(index);
			    match final_value_res {
				None => {
				    ret_record.insert(
					name.to_string(),
					Value::Null
				    );
				}
				Some(final_value) => {
				    ret_record.insert(
					name.to_string(),
					Value::BigInt(BigInt::from_i64(final_value).unwrap())
				    );
				}
			    }
			}
			"NUMERIC" => {
			    let final_value_res =
				raw_record.get::<Option<i64>, usize>(index);
			    match final_value_res {
				None => {
                                    let final_value_res =
                                        raw_record.get::<Option<f64>, usize>(index);
                                    match final_value_res {
                                        None => {
                                            let final_value_res =
                                                raw_record.get::<Option<String>, usize>(index);
                                            match final_value_res {
                                                None => {
                                                    ret_record.insert(
                                                        name.to_string(),
                                                        Value::Null
                                                    );
                                                }
                                                Some(final_value) => {
                                                    ret_record.insert(
                                                        name.to_string(),
                                                        new_string_value(final_value.to_string())
                                                    );
                                                }
                                            }
                                        }
                                        Some(final_value) => {
                                            ret_record.insert(
                                                name.to_string(),
                                                Value::Float(final_value as f64)
                                            );
                                        }
                                    }
				}
				Some(final_value) => {
				    ret_record.insert(
					name.to_string(),
					Value::BigInt(BigInt::from_i64(final_value).unwrap())
				    );
				}
			    }
			}
			"REAL" => {
			    let final_value_res =
				raw_record.get::<Option<f64>, usize>(index);
			    match final_value_res {
				None => {
				    ret_record.insert(
					name.to_string(),
					Value::Null
				    );
				}
				Some(final_value) => {
				    ret_record.insert(
					name.to_string(),
					Value::Float(final_value as f64)
				    );
				}
			    }
			}
			_ => {
			    self.print_error("unable to process database field type");
			    return 0;
			}
		    }
		}
		records.push_back(Value::Hash(Rc::new(RefCell::new(ret_record))));
	    }
	    self.stack.push(Value::List(Rc::new(RefCell::new(records))));
	    return 1;
        })
    }

    /// Takes a database statement and a list of parameters (which can
    /// be empty).  Executes the statement using those parameters and
    /// returns the results as a list of hashes.
    /// (For now, this fetches and returns all of the results in one
    /// go, in a list.  This may change so as to use a generator at
    /// some point, if the lifetime issues can be worked around in
    /// some way.)
    pub fn core_db_exec(&mut self) -> i32 {
        if self.stack.len() < 2 {
            self.print_error("db.exec requires two arguments");
            return 0;
        }

        let params = self.stack.pop().unwrap();
        let sv = self.stack.pop().unwrap();

        match (sv, params) {
            (Value::DBStatementMySQL(ref mut dbsv), Value::List(lst)) => {
                return self.db_exec_mysql(dbsv, lst);
            }
            (Value::DBStatementPostgres(ref mut dbsv), Value::List(lst)) => {
                return self.db_exec_postgres(dbsv, lst);
            }
            (Value::DBStatementSQLite(ref mut dbsv), Value::List(lst)) => {
                return self.db_exec_sqlite(dbsv, lst);
            }
            (Value::DBStatementMySQL(_), _) => {
                self.print_error("second db.exec argument must be list");
                return 0;
            }
            (Value::DBStatementPostgres(_), _) => {
                self.print_error("second db.exec argument must be list");
                return 0;
            }
            (Value::DBStatementSQLite(_), _) => {
                self.print_error("second db.exec argument must be list");
                return 0;
            }
            _ => {
                self.print_error("first db.exec argument must be database statement");
                return 0;
            }
        }
    }
}
