use num::FromPrimitive;
use num::ToPrimitive;
use num_bigint::BigInt;
use num_traits::Signed;

use chunk::{Value};
use vm::*;

/// Convert an i32 to a bigint value.
fn int_to_bigint(i: i32) -> Value {
    Value::BigInt(BigInt::from_i32(i).unwrap())
}

/// Convert a bigint to a floating-point value.
fn bigint_to_float(i: &BigInt) -> Value {
    Value::Float(FromPrimitive::from_u64(i.to_u64().unwrap()).unwrap())
}

/// Convert an i32 to a floating-point value.
fn int_to_float(i: i32) -> Value {
    Value::Float(FromPrimitive::from_i32(i).unwrap())
}

/// Add two integers together and return the result value.  Promote to
/// bigint if the value cannot be stored in an i32.
fn add_ints(n1: i32, n2: i32) -> Value {
    match n1.checked_add(n2) {
        Some(n3) => Value::Int(n3),
        None => {
            let n1_bigint = BigInt::from_i32(n1).unwrap();
            Value::BigInt(n1_bigint + n2)
        }
    }
}

/// Subtract one integer from another and return the result value.
/// Promote to bigint if the value cannot be stored in an i32.
fn subtract_ints(n1: i32, n2: i32) -> Value {
    match n2.checked_sub(n1) {
        Some(n3) => Value::Int(n3),
        None => {
            let n2_bigint = BigInt::from_i32(n2).unwrap();
            Value::BigInt(n2_bigint - n1)
        }
    }
}

/// Multiply two integers together and return the result value.
/// Promote to bigint if the value cannot be stored in an i32.
fn multiply_ints(n1: i32, n2: i32) -> Value {
    match n1.checked_mul(n2) {
        Some(n3) => Value::Int(n3),
        None => {
            let n1_bigint = BigInt::from_i32(n1).unwrap();
            Value::BigInt(n1_bigint * n2)
        }
    }
}

/// Divide one integer by anotherand return the result value.  Promote
/// to bigint if the value cannot be stored in an i32.
fn divide_ints(n1: i32, n2: i32) -> Value {
    match n2.checked_div(n1) {
        Some(n3) => Value::Int(n3),
        None => {
            let n2_bigint = BigInt::from_i32(n2).unwrap();
            Value::BigInt(n2_bigint / n1)
        }
    }
}

impl VM {
    /// Helper function for adding two values together and placing the
    /// result onto the stack.  Returns an integer indicating whether
    /// the values were able to be added together.
    fn opcode_add_inner(&mut self, v1: &Value, v2: &Value) -> i32 {
        match (&*v1, &*v2) {
            (Value::BigInt(n1), Value::BigInt(n2)) => {
                let n3 = Value::BigInt(n1 + n2);
                self.stack.push(n3);
                return 1;
            }
            (Value::BigInt(_), Value::Int(n2)) => {
                return self.opcode_add_inner(v1, &int_to_bigint(*n2));
            }
            (Value::Int(n1), Value::BigInt(_)) => {
                return self.opcode_add_inner(&int_to_bigint(*n1), v2);
            }
            (Value::Int(n1), Value::Int(n2)) => {
                self.stack.push(add_ints(*n1, *n2));
                return 1;
            }
            (Value::Float(n1), Value::Float(n2)) => {
                self.stack.push(Value::Float(n1 + n2));
                return 1;
            }
            (Value::BigInt(n1), Value::Float(_)) => {
                return self.opcode_add_inner(&bigint_to_float(n1), v2);
            }
            (Value::Float(_), Value::BigInt(n2)) => {
                return self.opcode_add_inner(v1, &bigint_to_float(n2));
            }
            (Value::Int(n1), Value::Float(_)) => {
                return self.opcode_add_inner(&int_to_float(*n1), v2);
            }
            (Value::Float(_), Value::Int(n2)) => {
                return self.opcode_add_inner(v1, &int_to_float(*n2));
            }
            (_, _) => {
                let n1_opt = v1.to_int();
                let n2_opt = v2.to_int();
                match (n1_opt, n2_opt) {
                    (Some(n1), Some(n2)) => {
                        self.stack.push(add_ints(n1, n2));
                        return 1;
                    }
                    _ => {}
                }
                let n1_opt = v1.to_bigint();
                let n2_opt = v2.to_bigint();
                match (n1_opt, n2_opt) {
                    (Some(n1), Some(n2)) => {
                        self.stack.push(Value::BigInt(n1 + n2));
                        return 1;
                    }
                    _ => {}
                }
                let n1_opt = v1.to_float();
                let n2_opt = v2.to_float();
                match (n1_opt, n2_opt) {
                    (Some(n1), Some(n2)) => {
                        self.stack.push(Value::Float(n1 + n2));
                        return 1;
                    }
                    _ => {}
                }
                return 0;
            }
        }
    }

    /// Takes two values as its arguments, adds them together, and
    /// places the result onto the stack.
    pub fn opcode_add(&mut self) -> i32 {
        let len = self.stack.len();
        if len < 2 {
            self.print_error("+ requires two arguments");
            return 0;
        }

        let v1_rr = self.stack.pop().unwrap();
        let mut done = false;
        match (&v1_rr, self.stack.get_mut(len - 2).unwrap()) {
            (Value::Int(n1), Value::Int(ref mut n2)) => {
                *n2 = *n2 + n1;
                done = true;
            }
            (_, _) => {}
        };

        if !done {
            let v2_rr = self.stack.pop().unwrap();

            let res = self.opcode_add_inner(&v1_rr, &v2_rr);
            if res == 0 {
                self.print_error("+ requires two numbers");
                return 0;
            }
        }

        return 1;
    }

    /// Helper function for subtracting two values and placing the
    /// result onto the stack.  Returns an integer indicating whether
    /// the values were able to be subtracted.
    fn opcode_subtract_inner(&mut self, v1: &Value, v2: &Value) -> i32 {
        match (&*v1, &*v2) {
            (Value::BigInt(n1), Value::BigInt(n2)) => {
                let n3 = Value::BigInt(n2 - n1);
                self.stack.push(n3);
                return 1;
            }
            (Value::BigInt(_), Value::Int(n2)) => {
                return self.opcode_subtract_inner(v1, &int_to_bigint(*n2));
            }
            (Value::Int(n1), Value::BigInt(_)) => {
                return self.opcode_subtract_inner(&int_to_bigint(*n1), v2);
            }
            (Value::Int(n1), Value::Int(n2)) => {
                self.stack.push(subtract_ints(*n1, *n2));
                return 1;
            }
            (Value::Float(n1), Value::Float(n2)) => {
                self.stack.push(Value::Float(n2 - n1));
                return 1;
            }
            (Value::BigInt(n1), Value::Float(_)) => {
                return self.opcode_subtract_inner(&bigint_to_float(n1), v2);
            }
            (Value::Float(_), Value::BigInt(n2)) => {
                return self.opcode_subtract_inner(v1, &bigint_to_float(n2));
            }
            (Value::Int(n1), Value::Float(_)) => {
                return self.opcode_subtract_inner(&int_to_float(*n1), v2);
            }
            (Value::Float(_), Value::Int(n2)) => {
                return self.opcode_subtract_inner(v1, &int_to_float(*n2));
            }
            (_, _) => {
                let n1_opt = v1.to_int();
                let n2_opt = v2.to_int();
                match (n1_opt, n2_opt) {
                    (Some(n1), Some(n2)) => {
                        self.stack.push(subtract_ints(n1, n2));
                        return 1;
                    }
                    _ => {}
                }
                let n1_opt = v1.to_bigint();
                let n2_opt = v2.to_bigint();
                match (n1_opt, n2_opt) {
                    (Some(n1), Some(n2)) => {
                        self.stack.push(Value::BigInt(n2 - n1));
                        return 1;
                    }
                    _ => {}
                }
                let n1_opt = v1.to_float();
                let n2_opt = v2.to_float();
                match (n1_opt, n2_opt) {
                    (Some(n1), Some(n2)) => {
                        self.stack.push(Value::Float(n2 - n1));
                        return 1;
                    }
                    _ => {}
                }
                return 0;
            }
        }
    }

    /// Takes two values as its arguments, subtracts them, and places
    /// the result onto the stack.
    pub fn opcode_subtract(&mut self) -> i32 {
        let len = self.stack.len();
        if len < 2 {
            self.print_error("- requires two arguments");
            return 0;
        }

        let v1_rr = self.stack.pop().unwrap();
        let mut done = false;
        match (&v1_rr, self.stack.get_mut(len - 2).unwrap()) {
            (Value::Int(n1), Value::Int(ref mut n2)) => {
                *n2 = *n2 - n1;
                done = true;
            }
            (_, _) => {}
        };

        if !done {
            let v2_rr = self.stack.pop().unwrap();

            let res = self.opcode_subtract_inner(&v1_rr, &v2_rr);
            if res == 0 {
                self.print_error("- requires two numbers");
                return 0;
            }
        }

        return 1;
    }

    /// Helper function for multiplying two values together and
    /// placing the result onto the stack.  Returns an integer
    /// indicating whether the values were able to be multiplied
    /// together.
    fn opcode_multiply_inner(&mut self, v1: &Value, v2: &Value) -> i32 {
        match (&*v1, &*v2) {
            (Value::BigInt(n1), Value::BigInt(n2)) => {
                let n3 = Value::BigInt(n1 * n2);
                self.stack.push(n3);
                return 1;
            }
            (Value::BigInt(_), Value::Int(n2)) => {
                return self.opcode_multiply_inner(v1, &int_to_bigint(*n2));
            }
            (Value::Int(n1), Value::BigInt(_)) => {
                return self.opcode_multiply_inner(&int_to_bigint(*n1), v2);
            }
            (Value::Int(n1), Value::Int(n2)) => {
                self.stack.push(multiply_ints(*n1, *n2));
                return 1;
            }
            (Value::Float(n1), Value::Float(n2)) => {
                self.stack.push(Value::Float(n1 * n2));
                return 1;
            }
            (Value::BigInt(n1), Value::Float(_)) => {
                return self.opcode_multiply_inner(&bigint_to_float(n1), v2);
            }
            (Value::Float(_), Value::BigInt(n2)) => {
                return self.opcode_multiply_inner(v1, &bigint_to_float(n2));
            }
            (Value::Int(n1), Value::Float(_)) => {
                return self.opcode_multiply_inner(&int_to_float(*n1), v2);
            }
            (Value::Float(_), Value::Int(n2)) => {
                return self.opcode_multiply_inner(v1, &int_to_float(*n2));
            }
            (_, _) => {
                let n1_opt = v1.to_int();
                let n2_opt = v2.to_int();
                match (n1_opt, n2_opt) {
                    (Some(n1), Some(n2)) => {
                        self.stack.push(multiply_ints(n1, n2));
                        return 1;
                    }
                    _ => {}
                }
                let n1_opt = v1.to_bigint();
                let n2_opt = v2.to_bigint();
                match (n1_opt, n2_opt) {
                    (Some(n1), Some(n2)) => {
                        self.stack.push(Value::BigInt(n1 * n2));
                        return 1;
                    }
                    _ => {}
                }
                let n1_opt = v1.to_float();
                let n2_opt = v2.to_float();
                match (n1_opt, n2_opt) {
                    (Some(n1), Some(n2)) => {
                        self.stack.push(Value::Float(n1 * n2));
                        return 1;
                    }
                    _ => {}
                }
                return 0;
            }
        }
    }

    /// Takes two values as its arguments, multiplies them together,
    /// and places the result onto the stack.
    pub fn opcode_multiply(&mut self) -> i32 {
        let len = self.stack.len();
        if len < 2 {
            self.print_error("* requires two arguments");
            return 0;
        }

        let v1_rr = self.stack.pop().unwrap();
        let mut done = false;
        match (&v1_rr, self.stack.get_mut(len - 2).unwrap()) {
            (Value::Int(n1), Value::Int(ref mut n2)) => {
                *n2 = *n2 * n1;
                done = true;
            }
            (_, _) => {}
        };

        if !done {
            let v2_rr = self.stack.pop().unwrap();

            let res = self.opcode_multiply_inner(&v1_rr, &v2_rr);
            if res == 0 {
                self.print_error("* requires two numbers");
                return 0;
            }
        }

        return 1;
    }

    /// Helper function for dividing two values and placing the result
    /// onto the stack.  Returns an integer indicating whether the
    /// values were able to be divided.
    fn opcode_divide_inner(&mut self, v1: &Value, v2: &Value) -> i32 {
        match (&*v1, &*v2) {
            (Value::BigInt(n1), Value::BigInt(n2)) => {
                let n3 = Value::BigInt(n2 / n1);
                self.stack.push(n3);
                return 1;
            }
            (Value::BigInt(_), Value::Int(n2)) => {
                return self.opcode_divide_inner(v1, &int_to_bigint(*n2));
            }
            (Value::Int(n1), Value::BigInt(_)) => {
                return self.opcode_divide_inner(&int_to_bigint(*n1), v2);
            }
            (Value::Int(n1), Value::Int(n2)) => {
                self.stack.push(divide_ints(*n1, *n2));
                return 1;
            }
            (Value::Float(n1), Value::Float(n2)) => {
                self.stack.push(Value::Float(n2 / n1));
                return 1;
            }
            (Value::BigInt(n1), Value::Float(_)) => {
                return self.opcode_divide_inner(&bigint_to_float(n1), v2);
            }
            (Value::Float(_), Value::BigInt(n2)) => {
                return self.opcode_divide_inner(v1, &bigint_to_float(n2));
            }
            (Value::Int(n1), Value::Float(_)) => {
                return self.opcode_divide_inner(&int_to_float(*n1), v2);
            }
            (Value::Float(_), Value::Int(n2)) => {
                return self.opcode_divide_inner(v1, &int_to_float(*n2));
            }
            (_, _) => {
                let n1_opt = v1.to_int();
                let n2_opt = v2.to_int();
                match (n1_opt, n2_opt) {
                    (Some(n1), Some(n2)) => {
                        self.stack.push(divide_ints(n1, n2));
                        return 1;
                    }
                    _ => {}
                }
                let n1_opt = v1.to_bigint();
                let n2_opt = v2.to_bigint();
                match (n1_opt, n2_opt) {
                    (Some(n1), Some(n2)) => {
                        self.stack.push(Value::BigInt(n2 / n1));
                        return 1;
                    }
                    _ => {}
                }
                let n1_opt = v1.to_float();
                let n2_opt = v2.to_float();
                match (n1_opt, n2_opt) {
                    (Some(n1), Some(n2)) => {
                        self.stack.push(Value::Float(n2 / n1));
                        return 1;
                    }
                    _ => {}
                }
                return 0;
            }
        }
    }

    /// Takes two values as its arguments, subtracts them, and places
    /// the result onto the stack.
    pub fn opcode_divide(&mut self) -> i32 {
        let len = self.stack.len();
        if len < 2 {
            self.print_error("/ requires two arguments");
            return 0;
        }

        let v1_rr = self.stack.pop().unwrap();
        let mut done = false;
        match (&v1_rr, self.stack.get_mut(len - 2).unwrap()) {
            (Value::Int(n1), Value::Int(ref mut n2)) => {
                *n2 = *n2 / n1;
                done = true;
            }
            (_, _) => {}
        };

        if !done {
            let v2_rr = self.stack.pop().unwrap();

            let res = self.opcode_divide_inner(&v1_rr, &v2_rr);
            if res == 0 {
                self.print_error("/ requires two numbers");
                return 0;
            }
        }

        return 1;
    }

    /// Helper function for checking whether two values are equal and
    /// placing a boolean onto the stack indicating whether they are
    /// equal.  Returns an integer indicating whether the values were
    /// able to be compared for equality.
    fn opcode_eq_inner(&mut self, v1: &Value, v2: &Value) -> i32 {
        match (&*v1, &*v2) {
            (Value::IpSet(s1), Value::IpSet(s2)) => {
                let res = *s1.borrow() == *s2.borrow();
                self.stack.push(Value::Bool(res));
                return 1;
            }
            (Value::BigInt(n1), Value::BigInt(n2)) => {
                let res = n1 == n2;
                self.stack.push(Value::Bool(res));
                return 1;
            }
            (Value::BigInt(_), Value::Int(n2)) => {
                return self.opcode_eq_inner(v1, &int_to_bigint(*n2));
            }
            (Value::Int(n1), Value::BigInt(_)) => {
                return self.opcode_eq_inner(&int_to_bigint(*n1), v2);
            }
            (Value::Int(n1), Value::Int(n2)) => {
                let res = n1 == n2;
                self.stack.push(Value::Bool(res));
                return 1;
            }
            (Value::BigInt(n1), Value::Float(_)) => {
                return self.opcode_eq_inner(&bigint_to_float(n1), v2);
            }
            (Value::Float(_), Value::BigInt(n2)) => {
                return self.opcode_eq_inner(v1, &bigint_to_float(n2));
            }
            (Value::Int(n1), Value::Float(_)) => {
                return self.opcode_eq_inner(&int_to_float(*n1), v2);
            }
            (Value::Float(_), Value::Int(n2)) => {
                return self.opcode_eq_inner(v1, &int_to_float(*n2));
            }
            (Value::Float(n1), Value::Float(n2)) => {
                let res = n1 == n2;
                self.stack.push(Value::Bool(res));
                return 1;
            }
            (Value::DateTimeNT(d1), Value::DateTimeNT(d2)) => {
                let res = d1 == d2;
                self.stack.push(Value::Bool(res));
                return 1;
            }
            (Value::DateTimeOT(d1), Value::DateTimeOT(d2)) => {
                let res = d1 == d2;
                self.stack.push(Value::Bool(res));
                return 1;
            }
            (Value::DateTimeOT(d1), Value::DateTimeNT(d2)) => {
                let res = d1 == d2;
                self.stack.push(Value::Bool(res));
                return 1;
            }
            (Value::DateTimeNT(d1), Value::DateTimeOT(d2)) => {
                let res = d1 == d2;
                self.stack.push(Value::Bool(res));
                return 1;
            }
            (_, _) => {
                let n1_opt = v1.to_int();
                let n2_opt = v2.to_int();
                match (n1_opt, n2_opt) {
                    (Some(n1), Some(n2)) => {
                        let res = n1 == n2;
                        self.stack.push(Value::Bool(res));
                        return 1;
                    }
                    _ => {}
                }
                let n1_opt = v1.to_bigint();
                let n2_opt = v2.to_bigint();
                match (n1_opt, n2_opt) {
                    (Some(n1), Some(n2)) => {
                        let res = n1 == n2;
                        self.stack.push(Value::Bool(res));
                        return 1;
                    }
                    _ => {}
                }
                let n1_opt = v1.to_float();
                let n2_opt = v2.to_float();
                match (n1_opt, n2_opt) {
                    (Some(n1), Some(n2)) => {
                        let res = n1 == n2;
                        self.stack.push(Value::Bool(res));
                        return 1;
                    }
                    _ => {}
                }

                let i1_str_opt: Option<&str>;
                to_str!(v1, i1_str_opt);
                
                let i2_str_opt: Option<&str>;
                to_str!(v2, i2_str_opt);

                match (i1_str_opt, i2_str_opt) {
                    (Some(n1), Some(n2)) => {
                        let res = n1 == n2;
                        self.stack.push(Value::Bool(res));
                        return 1;
                    }
                    _ => {}
                }
                return 0;
            }
        }
    }

    /// Takes two values as its arguments, compares them for equality,
    /// and places the result onto the stack.
    pub fn opcode_eq(&mut self) -> i32 {
        if self.stack.len() < 2 {
            self.print_error("= requires two arguments");
            return 0;
        }

        let v1_rr = self.stack.pop().unwrap();
        let v2_rr = self.stack.pop().unwrap();

        let res = self.opcode_eq_inner(&v1_rr, &v2_rr);
        if res == 0 {
            self.print_error("= requires two comparable values");
            return 0;
        }
        return 1;
    }

    /// Helper function for checking whether one value is greater than
    /// another, and placing a boolean onto the stack indicating
    /// whether that is so.  Returns an integer indicating whether the
    /// values were able to be compared.
    fn opcode_gt_inner(&mut self, v1: &Value, v2: &Value) -> i32 {
        match (&*v1, &*v2) {
            (Value::BigInt(n1), Value::BigInt(n2)) => {
                let res = n2 > n1;
                self.stack.push(Value::Bool(res));
                return 1;
            }
            (Value::BigInt(_), Value::Int(n2)) => {
                return self.opcode_gt_inner(v1, &int_to_bigint(*n2));
            }
            (Value::Int(n1), Value::BigInt(_)) => {
                return self.opcode_gt_inner(&int_to_bigint(*n1), v2);
            }
            (Value::Int(n1), Value::Int(n2)) => {
                let res = n2 > n1;
                self.stack.push(Value::Bool(res));
                return 1;
            }
            (Value::BigInt(n1), Value::Float(_)) => {
                return self.opcode_gt_inner(&bigint_to_float(n1), v2);
            }
            (Value::Float(_), Value::BigInt(n2)) => {
                return self.opcode_gt_inner(v1, &bigint_to_float(n2));
            }
            (Value::Int(n1), Value::Float(_)) => {
                return self.opcode_gt_inner(&int_to_float(*n1), v2);
            }
            (Value::Float(_), Value::Int(n2)) => {
                return self.opcode_gt_inner(v1, &int_to_float(*n2));
            }
            (Value::Float(n1), Value::Float(n2)) => {
                let res = n2 > n1;
                self.stack.push(Value::Bool(res));
                return 1;
            }
            (Value::DateTimeNT(d1), Value::DateTimeNT(d2)) => {
                let res = d2 > d1;
                self.stack.push(Value::Bool(res));
                return 1;
            }
            (Value::DateTimeOT(d1), Value::DateTimeOT(d2)) => {
                let res = d2 > d1;
                self.stack.push(Value::Bool(res));
                return 1;
            }
            (Value::DateTimeNT(d1), Value::DateTimeOT(d2)) => {
                let res = d2 > d1;
                self.stack.push(Value::Bool(res));
                return 1;
            }
            (Value::DateTimeOT(d1), Value::DateTimeNT(d2)) => {
                let res = d2 > d1;
                self.stack.push(Value::Bool(res));
                return 1;
            }
            (_, _) => {
                let n1_opt = v1.to_int();
                let n2_opt = v2.to_int();
                match (n1_opt, n2_opt) {
                    (Some(n1), Some(n2)) => {
                        let res = n2 > n1;
                        self.stack.push(Value::Bool(res));
                        return 1;
                    }
                    _ => {}
                }
                let n1_opt = v1.to_bigint();
                let n2_opt = v2.to_bigint();
                match (n1_opt, n2_opt) {
                    (Some(n1), Some(n2)) => {
                        let res = n2 > n1;
                        self.stack.push(Value::Bool(res));
                        return 1;
                    }
                    _ => {}
                }
                let n1_opt = v1.to_float();
                let n2_opt = v2.to_float();
                match (n1_opt, n2_opt) {
                    (Some(n1), Some(n2)) => {
                        let res = n2 > n1;
                        self.stack.push(Value::Bool(res));
                        return 1;
                    }
                    _ => {}
                }

                let i1_str_opt: Option<&str>;
                to_str!(v1, i1_str_opt);
                
                let i2_str_opt: Option<&str>;
                to_str!(v2, i2_str_opt);

                match (i1_str_opt, i2_str_opt) {
                    (Some(n1), Some(n2)) => {
                        let res = n2 > n1;
                        self.stack.push(Value::Bool(res));
                        return 1;
                    }
                    _ => {}
                }
                return 0;
            }
        }
    }

    /// Takes two values as its arguments, checks whether the first is
    /// greater than the second, and places the result onto the stack.
    pub fn opcode_gt(&mut self) -> i32 {
        if self.stack.len() < 2 {
            self.print_error("> requires two arguments");
            return 0;
        }

        let v1_rr = self.stack.pop().unwrap();
        let v2_rr = self.stack.pop().unwrap();

        let res = self.opcode_gt_inner(&v1_rr, &v2_rr);
        if res == 0 {
            self.print_error("> requires two comparable values");
            return 0;
        }
        return 1;
    }

    /// Helper function for checking whether one value is less than
    /// another, and placing a boolean onto the stack indicating
    /// whether that is so.  Returns an integer indicating whether the
    /// values were able to be compared.
    fn opcode_lt_inner(&mut self, v1: &Value, v2: &Value) -> i32 {
        match (&*v1, &*v2) {
            (Value::BigInt(n1), Value::BigInt(n2)) => {
                let res = n2 < n1;
                self.stack.push(Value::Bool(res));
                return 1;
            }
            (Value::BigInt(_), Value::Int(n2)) => {
                return self.opcode_lt_inner(v1, &int_to_bigint(*n2));
            }
            (Value::Int(n1), Value::BigInt(_)) => {
                return self.opcode_lt_inner(&int_to_bigint(*n1), v2);
            }
            (Value::Int(n1), Value::Int(n2)) => {
                let res = n2 < n1;
                self.stack.push(Value::Bool(res));
                return 1;
            }
            (Value::BigInt(n1), Value::Float(_)) => {
                return self.opcode_lt_inner(&bigint_to_float(n1), v2);
            }
            (Value::Float(_), Value::BigInt(n2)) => {
                return self.opcode_lt_inner(v1, &bigint_to_float(n2));
            }
            (Value::Int(n1), Value::Float(_)) => {
                return self.opcode_lt_inner(&int_to_float(*n1), v2);
            }
            (Value::Float(_), Value::Int(n2)) => {
                return self.opcode_lt_inner(v1, &int_to_float(*n2));
            }
            (Value::Float(n1), Value::Float(n2)) => {
                let res = n2 < n1;
                self.stack.push(Value::Bool(res));
                return 1;
            }
            (Value::DateTimeNT(d1), Value::DateTimeNT(d2)) => {
                let res = d2 < d1;
                self.stack.push(Value::Bool(res));
                return 1;
            }
            (Value::DateTimeOT(d1), Value::DateTimeOT(d2)) => {
                let res = d2 < d1;
                self.stack.push(Value::Bool(res));
                return 1;
            }
            (Value::DateTimeNT(d1), Value::DateTimeOT(d2)) => {
                let res = d2 < d1;
                self.stack.push(Value::Bool(res));
                return 1;
            }
            (Value::DateTimeOT(d1), Value::DateTimeNT(d2)) => {
                let res = d2 < d1;
                self.stack.push(Value::Bool(res));
                return 1;
            }
            (_, _) => {
                let n1_opt = v1.to_int();
                let n2_opt = v2.to_int();
                match (n1_opt, n2_opt) {
                    (Some(n1), Some(n2)) => {
                        let res = n2 < n1;
                        self.stack.push(Value::Bool(res));
                        return 1;
                    }
                    _ => {}
                }
                let n1_opt = v1.to_bigint();
                let n2_opt = v2.to_bigint();
                match (n1_opt, n2_opt) {
                    (Some(n1), Some(n2)) => {
                        let res = n2 < n1;
                        self.stack.push(Value::Bool(res));
                        return 1;
                    }
                    _ => {}
                }
                let n1_opt = v1.to_float();
                let n2_opt = v2.to_float();
                match (n1_opt, n2_opt) {
                    (Some(n1), Some(n2)) => {
                        let res = n2 < n1;
                        self.stack.push(Value::Bool(res));
                        return 1;
                    }
                    _ => {}
                }

                let i1_str_opt: Option<&str>;
                to_str!(v1, i1_str_opt);
                
                let i2_str_opt: Option<&str>;
                to_str!(v2, i2_str_opt);

                match (i1_str_opt, i2_str_opt) {
                    (Some(n1), Some(n2)) => {
                        let res = n2 < n1;
                        self.stack.push(Value::Bool(res));
                        return 1;
                    }
                    _ => {}
                }
                return 0;
            }
        }
    }

    /// Takes two values as its arguments, checks whether the first is
    /// less than the second, and places the result onto the stack.
    pub fn opcode_lt(&mut self) -> i32 {
        if self.stack.len() < 2 {
            self.print_error("< requires two arguments");
            return 0;
        }

        let v1_rr = self.stack.pop().unwrap();
        let v2_rr = self.stack.pop().unwrap();

        let res = self.opcode_lt_inner(&v1_rr, &v2_rr);
        if res == 0 {
            self.print_error("< requires two comparable values");
            return 0;
        }
        return 1;
    }

    /// Get the square root of a number.
    pub fn core_sqrt(&mut self) -> i32 {
        if self.stack.len() < 1 {
            self.print_error("sqrt requires one argument");
            return 0;
        }

        let value_rr = self.stack.pop().unwrap();
        let f_opt = value_rr.to_float();
        match f_opt {
            Some(f) => {
                let fs = f.sqrt();
                self.stack.push(Value::Float(fs));
                return 1;
            }
            None => {
                self.print_error("cannot convert argument to float");
                return 0;
            }
        }
    }

    /// Helper function for exponentiation.
    fn core_exp_inner(&mut self, v1: &Value, v2: &Value) -> i32 {
        match (&*v1, &*v2) {
            (Value::Int(n), Value::Int(exp)) => {
                if *exp < 0 {
                    self.print_error("exponent cannot be negative");
                    return 0;
                }
                let nn = (*n).checked_pow((*exp).try_into().unwrap());
                match nn {
                    Some(nnn) => {
                        self.stack.push(Value::Int(nnn));
                        return 1;
                    }
                    None => {
                        let bi = BigInt::from_i32(*n).unwrap();
                        let bb = bi.pow((*exp).try_into().unwrap());
                        self.stack.push(Value::BigInt(bb));
                        return 1;
                    }
                }
            }
            (Value::Float(f), Value::Int(exp)) => {
                if *exp < 0 {
                    self.print_error("exponent cannot be negative");
                    return 0;
                }
                let ff = (*f).powf((*exp).try_into().unwrap());
                self.stack.push(Value::Float(ff));
                return 1;
            }
            (Value::BigInt(bi), Value::Int(exp)) => {
                if *exp < 0 {
                    self.print_error("exponent cannot be negative");
                    return 0;
                }
                let bb = (*bi).pow((*exp).try_into().unwrap());
                self.stack.push(Value::BigInt(bb));
                return 1;
            }
            (Value::Int(n), Value::Float(exp)) => {
                if *exp < 0.0 {
                    self.print_error("exponent cannot be negative");
                    return 0;
                }
                let f = *n as f64;
                let ff = f.powf((*exp).try_into().unwrap());
                self.stack.push(Value::Float(ff));
                return 1;
            }
            (Value::Float(f), Value::Float(exp)) => {
                if *exp < 0.0 {
                    self.print_error("exponent cannot be negative");
                    return 0;
                }
                let ff = (*f).powf((*exp).try_into().unwrap());
                self.stack.push(Value::Float(ff));
                return 1;
            }
            (Value::BigInt(bi), Value::Float(exp)) => {
                if *exp < 0.0 {
                    self.print_error("exponent cannot be negative");
                    return 0;
                }
                let ff = (*bi).to_f64().unwrap().powf((*exp).try_into().unwrap());
                self.stack.push(Value::Float(ff));
                return 1;
            }
            (Value::Int(_), _) => {
                let n_opt = v2.to_int();
                match n_opt {
                    Some(n) => {
                        return self.core_exp_inner(v1, &Value::Int(n));
                    }
                    _ => {}
                }

                let f_opt = v2.to_float();
                match f_opt {
                    Some(f) => {
                        return self.core_exp_inner(v1, &Value::Float(f));
                    }
                    _ => {}
                }

                return 0;
            }
            (Value::BigInt(_), _) => {
                let n_opt = v2.to_int();
                match n_opt {
                    Some(n) => {
                        return self.core_exp_inner(v1, &Value::Int(n));
                    }
                    _ => {}
                }

                let f_opt = v2.to_float();
                match f_opt {
                    Some(f) => {
                        return self.core_exp_inner(v1, &Value::Float(f));
                    }
                    _ => {}
                }

                return 0;
            }
            (Value::Float(_), _) => {
                let n_opt = v2.to_int();
                match n_opt {
                    Some(n) => {
                        return self.core_exp_inner(v1, &Value::Int(n));
                    }
                    _ => {}
                }

                let f_opt = v2.to_float();
                match f_opt {
                    Some(f) => {
                        return self.core_exp_inner(v1, &Value::Float(f));
                    }
                    _ => {}
                }

                return 0;
            }
            (_, _) => {
                let n_opt = v1.to_int();
                match n_opt {
                    Some(n) => {
                        return self.core_exp_inner(&Value::Int(n), v2);
                    }
                    _ => {}
                }

                let f_opt = v1.to_float();
                match f_opt {
                    Some(f) => {
                        return self.core_exp_inner(&Value::Float(f), v2);
                    }
                    _ => {}
                }

                let bi_opt = v1.to_bigint();
                match bi_opt {
                    Some(bi) => {
                        return self.core_exp_inner(&Value::BigInt(bi), v2);
                    }
                    _ => {}
                }

                return 0;
            }
        }
    }

    /// Raise the first argument to the second argument.
    pub fn core_exp(&mut self) -> i32 {
        if self.stack.len() < 2 {
            self.print_error("exp requires two arguments");
            return 0;
        }

        let exp_rr = self.stack.pop().unwrap();
        let base_rr = self.stack.pop().unwrap();

        let res = self.core_exp_inner(&base_rr, &exp_rr);
        if res == 0 {
            self.print_error("unhandled exp arguments");
            return 0;
        }

        return 1;
    }

    /// Get the absolute value of the argument.
    pub fn core_abs(&mut self) -> i32 {
        if self.stack.len() < 1 {
            self.print_error("sqrt requires one argument");
            return 0;
        }

        let value_rr = self.stack.pop().unwrap();
        match value_rr {
            Value::Int(n) => {
                let nn = n.abs();
                self.stack.push(Value::Int(nn));
                return 1;
            }
            Value::Float(f) => {
                let ff = f.abs();
                self.stack.push(Value::Float(ff));
                return 1;
            }
            Value::BigInt(bi) => {
                let bb = bi.abs();
                self.stack.push(Value::BigInt(bb));
                return 1;
            }
            _ => {}
        }

        let n_opt = value_rr.to_int();
        match n_opt {
            Some(n) => {
                let nn = n.abs();
                self.stack.push(Value::Int(nn));
                return 1;
            }
            _ => {}
        }

        let bi_opt = value_rr.to_bigint();
        match bi_opt {
            Some(bi) => {
                let bb = bi.abs();
                self.stack.push(Value::BigInt(bb));
                return 1;
            }
            _ => {}
        }

        let f_opt = value_rr.to_float();
        match f_opt {
            Some(f) => {
                let ff = f.abs();
                self.stack.push(Value::Float(ff));
                return 1;
            }
            _ => {}
        }

        self.print_error("unhandled abs argument");
        return 0;
    }
}
