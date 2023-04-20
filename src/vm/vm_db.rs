use crate::chunk::DBConnection;
use crate::chunk::DBStatement;
use crate::chunk::Value;
use crate::vm::*;
use futures::executor::block_on;
use sqlx::Column;
use sqlx::Row;
use sqlx::TypeInfo;

impl VM {
    pub fn core_db_conn(&mut self) -> i32 {
        if self.stack.len() < 5 {
            self.print_error("db.conn requires five arguments");
            return 0;
        }

        let pass = self.stack.pop().unwrap();
        let user = self.stack.pop().unwrap();
        let db = self.stack.pop().unwrap();
        let host = self.stack.pop().unwrap();
        let dbtype = self.stack.pop().unwrap();

        let dbtype_str_opt: Option<&str>;
        to_str!(dbtype, dbtype_str_opt);
        let pass_str_opt: Option<&str>;
        to_str!(pass, pass_str_opt);
        let user_str_opt: Option<&str>;
        to_str!(user, user_str_opt);
        let db_str_opt: Option<&str>;
        to_str!(db, db_str_opt);
        let host_str_opt: Option<&str>;
        to_str!(host, host_str_opt);

        match (dbtype_str_opt, host_str_opt, db_str_opt, user_str_opt, pass_str_opt) {
            (Some(dbtype_str), Some(host_str), Some(db_str), Some(user_str), Some(pass_str)) => {
                let url = format!("{}://{}:{}@{}/{}",
                    dbtype_str, user_str, pass_str, host_str, db_str);
                let pool = sqlx::Pool::connect_lazy(&url).unwrap();
                let dbc = DBConnection::new(pool);
                let dbcv = Value::DBConnection(Rc::new(RefCell::new(dbc)));
                self.stack.push(dbcv);
                return 1;
            }
            _ => {
                self.print_error("bad arguments to db.conn");
                return 0;
            }
        }
    }

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
            (Value::DBConnection(dbc), Some(sql_str)) => {
                let dbs = DBStatement::new(dbc.borrow().pool.clone(), sql_str.to_string());
                let dbsv = Value::DBStatement(Rc::new(RefCell::new(dbs)));
                self.stack.push(dbsv);
                return 1;
            }
            (Value::DBConnection(_), _) => {
                self.print_error("second db.prep argument must be string");
                return 0;
            }
            _ => {
                self.print_error("first db.prep argument must be database connection");
                return 0;
            }
        }
    }

    pub fn core_db_exec(&mut self) -> i32 {
        let res = block_on(async {
            if self.stack.len() < 2 {
                self.print_error("db.exec requires two arguments");
                return 0;
            }

            let params = self.stack.pop().unwrap();
            let sv = self.stack.pop().unwrap();

            match (sv, params) {
                (Value::DBStatement(ref mut dbsv), Value::List(lst)) => {
                    let mut dbsvb = dbsv.borrow_mut();
                    let query = dbsvb.query.clone();
                    let pool = &mut dbsvb.pool;
                    let mut conn = pool.acquire().await.unwrap();
                    let mut queryo = sqlx::query(&query);
                    let lb = lst.borrow();
                    for i in lb.iter() {
                        match i {
                            Value::String(s) => {
                                queryo =
                                    queryo.bind(s.borrow().string.clone());
                                continue;
                            }
                            Value::Int(n) => {
                                queryo = queryo.bind(n);
                                continue;
                            }
                            _ => {}
                        }
                        let i_str_opt: Option<&str>;
                        to_str!(i, i_str_opt);
                        match i_str_opt {
                            Some(s) => {
                                queryo = queryo.bind(s.to_string());
                            }
                            _ => {
                                self.print_error("durp");
                                return 0;
                            }
                        }
                    }
                    let qr = queryo.fetch_all(&mut conn).await;
                    let mut vv = VecDeque::new();
                    for recw in qr {
                        for rec in recw {
                            let mut newrec = IndexMap::new();
                            for c in rec.columns() {
                                let name = c.name();
                                let index = c.ordinal();
                                let ti = c.type_info();
                                match ti.name() {
                                    "VARCHAR" => {
                                        let res = rec.get::<String, usize>(index);
                                        newrec.insert(name.to_string(),
                                        Value::String(Rc::new(RefCell::new(StringTriple::new(res.to_string(),
                                            None)))));
                                    }
                                    "INT" => {
                                        let res = rec.get::<i32, usize>(index);
                                        newrec.insert(name.to_string(),
                                                      Value::Int(res));
                                    }
                                    _ => {}
                                }
                            }
                            vv.push_back(Value::Hash(Rc::new(RefCell::new(newrec))));
                        }
                        self.stack.push(Value::List(Rc::new(RefCell::new(vv))));
                        break;
                    }

                    return 1;
                }
                _ => {
                    self.print_error("bad arguments to db.exec");
                    return 0;
                }
            }
        });
        return res;
    }
}
