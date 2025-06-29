use num::FromPrimitive;
use num::ToPrimitive;
use num::Integer;
use num_bigint::BigInt;
use num_traits::Signed;

use crate::chunk::Value;
use crate::vm::*;

/// Convert an i32 to a bigint value.
fn int_to_bigint(i: i32) -> Value {
    Value::BigInt(BigInt::from_i32(i).unwrap())
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

/// Divide one integer by another and return the result value.  Promote
/// to bigint if the value cannot be stored in an i32.
fn divide_ints(n1: i32, n2: i32) -> Option<Value> {
    if n1 == 0 {
        return None; // Division by zero
    }
    match n2.checked_div(n1) {
        Some(n3) => Some(Value::Int(n3)),
        None => {
            let n2_bigint = BigInt::from_i32(n2).unwrap();
            Some(Value::BigInt(n2_bigint / n1))
        }
    }
}

/// Divide one integer by another and return the remainder.  Promote
/// to bigint if the value cannot be stored in an i32.
fn remainder_ints(n1: i32, n2: i32) -> Value {
    match n2.checked_rem(n1) {
        Some(n3) => Value::Int(n3),
        None => {
            let n2_bigint = BigInt::from_i32(n2).unwrap();
            let n1_bigint = BigInt::from_i32(n1).unwrap();
            let (_, remainder) = n2_bigint.div_rem(&n1_bigint);
            Value::BigInt(remainder)
        }
    }
}

impl VM {
    /// Helper function to handle errors appropriately based on try mode.
    /// Returns 1 in try mode (continue execution), 0 otherwise (terminate).
    fn handle_arithmetic_error(&mut self, error: &str) -> i32 {
        self.print_error(error);
        if self.try_mode {
            1  // Continue execution in try mode
        } else {
            0  // Terminate execution
        }
    }
    /// Helper function for adding two values together and placing the
    /// result onto the stack.  Returns an integer indicating whether
    /// the values were able to be added together.
    fn opcode_add_inner(&mut self, v1: &Value, v2: &Value) -> i32 {
        match (v1, v2) {
            (Value::BigInt(n1), Value::BigInt(n2)) => {
                let n3 = Value::BigInt(n1 + n2);
                self.stack.push(n3);
                1
            }
            (Value::BigInt(_), Value::Int(n2)) => self.opcode_add_inner(v1, &int_to_bigint(*n2)),
            (Value::Int(n1), Value::BigInt(_)) => self.opcode_add_inner(&int_to_bigint(*n1), v2),
            (Value::Int(n1), Value::Int(n2)) => {
                self.stack.push(add_ints(*n1, *n2));
                1
            }
            (Value::Float(n1), Value::Float(n2)) => {
                self.stack.push(Value::Float(n1 + n2));
                1
            }
            (Value::Int(n1), Value::Float(_)) => self.opcode_add_inner(&int_to_float(*n1), v2),
            (Value::Float(_), Value::Int(n2)) => self.opcode_add_inner(v1, &int_to_float(*n2)),
            (_, _) => {
                let n1_opt = v1.to_int();
                let n2_opt = v2.to_int();
                if let (Some(n1), Some(n2)) = (n1_opt, n2_opt) {
                    self.stack.push(add_ints(n1, n2));
                    return 1;
                }
                let n1_opt = v1.to_bigint();
                let n2_opt = v2.to_bigint();
                if let (Some(n1), Some(n2)) = (n1_opt, n2_opt) {
                    self.stack.push(Value::BigInt(n1 + n2));
                    return 1;
                }
                let n1_opt = v1.to_float();
                let n2_opt = v2.to_float();
                if let (Some(n1), Some(n2)) = (n1_opt, n2_opt) {
                    self.stack.push(Value::Float(n1 + n2));
                    return 1;
                }
                0
            }
        }
    }

    /// Takes two values as its arguments, adds them together, and
    /// places the result onto the stack.
    pub fn opcode_add(&mut self) -> i32 {
        let len = self.stack.len();
        if len < 2 {
            return self.handle_arithmetic_error("+ requires two arguments");
        }

        let v1_rr = self.stack.pop().unwrap();
        let mut done = false;
        if let (Value::Int(n1), Value::Int(ref mut n2)) =
            (&v1_rr, self.stack.get_mut(len - 2).unwrap())
        {
            let v3 = add_ints(*n1, *n2);
            if let Value::Int(n3) = v3 {
                *n2 = n3;
                done = true;
            }
        }

        if !done {
            let v2_rr = self.stack.pop().unwrap();

            let res = self.opcode_add_inner(&v1_rr, &v2_rr);
            if res == 0 {
                return self.handle_arithmetic_error("+ requires two numbers");
            }
        }

        1
    }

    /// Helper function for subtracting two values and placing the
    /// result onto the stack.  Returns an integer indicating whether
    /// the values were able to be subtracted.
    fn opcode_subtract_inner(&mut self, v1: &Value, v2: &Value) -> i32 {
        match (v1, v2) {
            (Value::BigInt(n1), Value::BigInt(n2)) => {
                let n3 = Value::BigInt(n2 - n1);
                self.stack.push(n3);
                1
            }
            (Value::BigInt(_), Value::Int(n2)) => {
                self.opcode_subtract_inner(v1, &int_to_bigint(*n2))
            }
            (Value::Int(n1), Value::BigInt(_)) => {
                self.opcode_subtract_inner(&int_to_bigint(*n1), v2)
            }
            (Value::Int(n1), Value::Int(n2)) => {
                self.stack.push(subtract_ints(*n1, *n2));
                1
            }
            (Value::Float(n1), Value::Float(n2)) => {
                self.stack.push(Value::Float(n2 - n1));
                1
            }
            (Value::Int(n1), Value::Float(_)) => self.opcode_subtract_inner(&int_to_float(*n1), v2),
            (Value::Float(_), Value::Int(n2)) => self.opcode_subtract_inner(v1, &int_to_float(*n2)),
            (_, _) => {
                let n1_opt = v1.to_int();
                let n2_opt = v2.to_int();
                if let (Some(n1), Some(n2)) = (n1_opt, n2_opt) {
                    self.stack.push(subtract_ints(n1, n2));
                    return 1;
                }
                let n1_opt = v1.to_bigint();
                let n2_opt = v2.to_bigint();
                if let (Some(n1), Some(n2)) = (n1_opt, n2_opt) {
                    self.stack.push(Value::BigInt(n2 - n1));
                    return 1;
                }
                let n1_opt = v1.to_float();
                let n2_opt = v2.to_float();
                if let (Some(n1), Some(n2)) = (n1_opt, n2_opt) {
                    self.stack.push(Value::Float(n2 - n1));
                    return 1;
                }
                0
            }
        }
    }

    /// Takes two values as its arguments, subtracts them, and places
    /// the result onto the stack.
    pub fn opcode_subtract(&mut self) -> i32 {
        let len = self.stack.len();
        if len < 2 {
            return self.handle_arithmetic_error("- requires two arguments");
        }

        let v1_rr = self.stack.pop().unwrap();
        let mut done = false;
        if let (Value::Int(n1), Value::Int(ref mut n2)) =
            (&v1_rr, self.stack.get_mut(len - 2).unwrap())
        {
            let v3 = subtract_ints(*n1, *n2);
            if let Value::Int(n3) = v3 {
                *n2 = n3;
                done = true;
            }
        }

        if !done {
            let v2_rr = self.stack.pop().unwrap();

            let res = self.opcode_subtract_inner(&v1_rr, &v2_rr);
            if res == 0 {
                return self.handle_arithmetic_error("- requires two numbers");
            }
        }

        1
    }

    /// Helper function for multiplying two values together and
    /// placing the result onto the stack.  Returns an integer
    /// indicating whether the values were able to be multiplied
    /// together.
    fn opcode_multiply_inner(&mut self, v1: &Value, v2: &Value) -> i32 {
        match (v1, v2) {
            (Value::BigInt(n1), Value::BigInt(n2)) => {
                let n3 = Value::BigInt(n1 * n2);
                self.stack.push(n3);
                1
            }
            (Value::BigInt(_), Value::Int(n2)) => {
                self.opcode_multiply_inner(v1, &int_to_bigint(*n2))
            }
            (Value::Int(n1), Value::BigInt(_)) => {
                self.opcode_multiply_inner(&int_to_bigint(*n1), v2)
            }
            (Value::Int(n1), Value::Int(n2)) => {
                self.stack.push(multiply_ints(*n1, *n2));
                1
            }
            (Value::Float(n1), Value::Float(n2)) => {
                self.stack.push(Value::Float(n1 * n2));
                1
            }
            (Value::Int(n1), Value::Float(_)) => self.opcode_multiply_inner(&int_to_float(*n1), v2),
            (Value::Float(_), Value::Int(n2)) => self.opcode_multiply_inner(v1, &int_to_float(*n2)),
            (_, _) => {
                let n1_opt = v1.to_int();
                let n2_opt = v2.to_int();
                if let (Some(n1), Some(n2)) = (n1_opt, n2_opt) {
                    self.stack.push(multiply_ints(n1, n2));
                    return 1;
                }
                let n1_opt = v1.to_bigint();
                let n2_opt = v2.to_bigint();
                if let (Some(n1), Some(n2)) = (n1_opt, n2_opt) {
                    self.stack.push(Value::BigInt(n1 * n2));
                    return 1;
                }
                let n1_opt = v1.to_float();
                let n2_opt = v2.to_float();
                if let (Some(n1), Some(n2)) = (n1_opt, n2_opt) {
                    self.stack.push(Value::Float(n1 * n2));
                    return 1;
                }
                0
            }
        }
    }

    /// Takes two values as its arguments, multiplies them together,
    /// and places the result onto the stack.
    pub fn opcode_multiply(&mut self) -> i32 {
        let len = self.stack.len();
        if len < 2 {
            return self.handle_arithmetic_error("* requires two arguments");
        }

        let v1_rr = self.stack.pop().unwrap();
        let mut done = false;
        if let (Value::Int(n1), Value::Int(ref mut n2)) =
            (&v1_rr, self.stack.get_mut(len - 2).unwrap())
        {
            let v3 = multiply_ints(*n1, *n2);
            if let Value::Int(n3) = v3 {
                *n2 = n3;
                done = true;
            }
        }

        if !done {
            let v2_rr = self.stack.pop().unwrap();

            let res = self.opcode_multiply_inner(&v1_rr, &v2_rr);
            if res == 0 {
                return self.handle_arithmetic_error("* requires two numbers");
            }
        }

        1
    }

    /// Helper function for dividing two values and placing the result
    /// onto the stack.  Returns an integer indicating whether the
    /// values were able to be divided.
    fn opcode_divide_inner(&mut self, v1: &Value, v2: &Value) -> i32 {
        match (v1, v2) {
            (Value::BigInt(n1), Value::BigInt(n2)) => {
                if *n1 == BigInt::from_i32(0).unwrap() {
                    return -1; // Division by zero
                }
                let n3 = Value::BigInt(n2 / n1);
                self.stack.push(n3);
                1
            }
            (Value::BigInt(_), Value::Int(n2)) => self.opcode_divide_inner(v1, &int_to_bigint(*n2)),
            (Value::Int(n1), Value::BigInt(_)) => self.opcode_divide_inner(&int_to_bigint(*n1), v2),
            (Value::Int(n1), Value::Int(n2)) => {
                match divide_ints(*n1, *n2) {
                    Some(result) => {
                        self.stack.push(result);
                        1
                    }
                    None => -1 // Division by zero
                }
            }
            (Value::Float(n1), Value::Float(n2)) => {
                if *n1 == 0.0 {
                    return -1; // Division by zero
                }
                self.stack.push(Value::Float(n2 / n1));
                1
            }
            (Value::Int(n1), Value::Float(_)) => self.opcode_divide_inner(&int_to_float(*n1), v2),
            (Value::Float(_), Value::Int(n2)) => self.opcode_divide_inner(v1, &int_to_float(*n2)),
            (_, _) => {
                let n1_opt = v1.to_int();
                let n2_opt = v2.to_int();
                if let (Some(n1), Some(n2)) = (n1_opt, n2_opt) {
                    match divide_ints(n1, n2) {
                        Some(result) => {
                            self.stack.push(result);
                            return 1;
                        }
                        None => return -1 // Division by zero
                    }
                }
                let n1_opt = v1.to_bigint();
                let n2_opt = v2.to_bigint();
                if let (Some(n1), Some(n2)) = (n1_opt, n2_opt) {
                    self.stack.push(Value::BigInt(n2 / n1));
                    return 1;
                }
                let n1_opt = v1.to_float();
                let n2_opt = v2.to_float();
                if let (Some(n1), Some(n2)) = (n1_opt, n2_opt) {
                    if n1 == 0.0 {
                        return -1; // Division by zero
                    }
                    self.stack.push(Value::Float(n2 / n1));
                    return 1;
                }
                0
            }
        }
    }

    /// Takes two values as its arguments, subtracts them, and places
    /// the result onto the stack.
    pub fn opcode_divide(&mut self) -> i32 {
        let len = self.stack.len();
        if len < 2 {
            return self.handle_arithmetic_error("/ requires two arguments");
        }

        let v1_rr = self.stack.pop().unwrap();
        let mut done = false;

        if let (Value::Int(n1), Value::Int(ref mut n2)) =
            (&v1_rr, self.stack.get_mut(len - 2).unwrap())
        {
            match divide_ints(*n1, *n2) {
                Some(v3) => {
                    if let Value::Int(n3) = v3 {
                        *n2 = n3;
                        done = true;
                    }
                }
                None => {
                    return self.handle_arithmetic_error("/ requires two non-zero numbers");
                }
            }
        }

        if !done {
            let v2_rr = self.stack.pop().unwrap();

            let res = self.opcode_divide_inner(&v1_rr, &v2_rr);
            if res == 0 {
                return self.handle_arithmetic_error("/ requires two numbers");
            } else if res == -1 {
                return self.handle_arithmetic_error("/ requires two non-zero numbers");
            }
        }

        1
    }

    /// Helper function for dividing two values and placing the
    /// remainder onto the stack.  Returns an integer indicating
    /// whether the values were able to be divided.
    fn opcode_remainder_inner(&mut self, v1: &Value, v2: &Value) -> i32 {
        match (v1, v2) {
            (Value::BigInt(n1), Value::BigInt(n2)) => {
                let (_, remainder) = n2.div_rem(&n1);
                self.stack.push(Value::BigInt(remainder));
                1
            }
            (Value::BigInt(_), Value::Int(n2)) => self.opcode_remainder_inner(v1, &int_to_bigint(*n2)),
            (Value::Int(n1), Value::BigInt(_)) => self.opcode_remainder_inner(&int_to_bigint(*n1), v2),
            (Value::Int(n1), Value::Int(n2)) => {
                self.stack.push(remainder_ints(*n1, *n2));
                1
            }
            (_, _) => {
                let n1_opt = v1.to_int();
                let n2_opt = v2.to_int();
                if let (Some(n1), Some(n2)) = (n1_opt, n2_opt) {
                    self.stack.push(remainder_ints(n1, n2));
                    return 1;
                }
                let n1_opt = v1.to_bigint();
                let n2_opt = v2.to_bigint();
                if let (Some(n1), Some(n2)) = (n1_opt, n2_opt) {
                    let (_, remainder) = n2.div_rem(&n1);
                    self.stack.push(Value::BigInt(remainder));
                    return 1;
                }
                0
            }
        }
    }

    /// Takes two values as its arguments, subtracts them, and places
    /// the result onto the stack.
    pub fn opcode_remainder(&mut self) -> i32 {
        let len = self.stack.len();
        if len < 2 {
            self.print_error("% requires two arguments");
            return 0;
        }

        let v1_rr = self.stack.pop().unwrap();
        let mut done = false;

        if let (Value::Int(n1), Value::Int(ref mut n2)) =
            (&v1_rr, self.stack.get_mut(len - 2).unwrap())
        {
            let v3 = remainder_ints(*n1, *n2);
            if let Value::Int(n3) = v3 {
                *n2 = n3;
                done = true;
            }
        }

        if !done {
            let v2_rr = self.stack.pop().unwrap();

            let res = self.opcode_remainder_inner(&v1_rr, &v2_rr);
            if res == 0 {
                self.print_error("% requires two numbers");
                return 0;
            }
        }

        1
    }

    /// Helper function for checking whether two values are equal.
    /// Returns 1 if they are equal, 0 if they are not, and -1 if they
    /// cannot be compared.
    pub fn opcode_eq_inner(&mut self, v1: &Value, v2: &Value) -> i32 {
        match (v1, v2) {
            (Value::IpSet(s1), Value::IpSet(s2)) => {
                if *s1.borrow() == *s2.borrow() {
                    1
                } else {
                    0
                }
            }
            (Value::BigInt(n1), Value::BigInt(n2)) => {
                if n1 == n2 {
                    1
                } else {
                    0
                }
            }
            (Value::BigInt(_), Value::Int(n2)) => self.opcode_eq_inner(v1, &int_to_bigint(*n2)),
            (Value::Int(n1), Value::BigInt(_)) => self.opcode_eq_inner(&int_to_bigint(*n1), v2),
            (Value::Int(n1), Value::Int(n2)) => {
                if n1 == n2 {
                    1
                } else {
                    0
                }
            }
            (Value::Int(n1), Value::Float(_)) => self.opcode_eq_inner(&int_to_float(*n1), v2),
            (Value::Float(_), Value::Int(n2)) => self.opcode_eq_inner(v1, &int_to_float(*n2)),
            (Value::Float(n1), Value::Float(n2)) => {
                if n1 == n2 {
                    1
                } else {
                    0
                }
            }
            (Value::DateTimeNT(d1), Value::DateTimeNT(d2)) => {
                if d1 == d2 {
                    1
                } else {
                    0
                }
            }
            (Value::DateTimeOT(d1), Value::DateTimeOT(d2)) => {
                if d1 == d2 {
                    1
                } else {
                    0
                }
            }
            (Value::DateTimeOT(d1), Value::DateTimeNT(d2)) => {
                if d1 == d2 {
                    1
                } else {
                    0
                }
            }
            (Value::DateTimeNT(d1), Value::DateTimeOT(d2)) => {
                if d1 == d2 {
                    1
                } else {
                    0
                }
            }
            (_, _) => {
                let n1_opt = v1.to_int();
                let n2_opt = v2.to_int();
                if let (Some(n1), Some(n2)) = (n1_opt, n2_opt) {
                    return if n1 == n2 { 1 } else { 0 };
                }
                let n1_opt = v1.to_bigint();
                let n2_opt = v2.to_bigint();
                if let (Some(n1), Some(n2)) = (n1_opt, n2_opt) {
                    return if n1 == n2 { 1 } else { 0 };
                }
                let n1_opt = v1.to_float();
                let n2_opt = v2.to_float();
                if let (Some(n1), Some(n2)) = (n1_opt, n2_opt) {
                    return if n1 == n2 { 1 } else { 0 };
                }

                let i1_str_opt: Option<&str>;
                to_str!(v1, i1_str_opt);

                let i2_str_opt: Option<&str>;
                to_str!(v2, i2_str_opt);

                if let (Some(n1), Some(n2)) = (i1_str_opt, i2_str_opt) {
                    return if n1 == n2 { 1 } else { 0 };
                }
                -1
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
        if res == 1 {
            self.stack.push(Value::Bool(true));
        } else if res == 0 {
            self.stack.push(Value::Bool(false));
        } else {
            self.print_error("= requires two comparable values");
            return 0;
        }
        1
    }

    /// Helper function for checking whether one value is greater than
    /// another.  Returns 1 if it is, 0 if it isn't, and -1 if the two
    /// values cannot be compared.
    pub fn opcode_gt_inner(&mut self, v1: &Value, v2: &Value) -> i32 {
        match (v1, v2) {
            (Value::BigInt(n1), Value::BigInt(n2)) => {
                if n2 > n1 {
                    1
                } else {
                    0
                }
            }
            (Value::BigInt(_), Value::Int(n2)) => self.opcode_gt_inner(v1, &int_to_bigint(*n2)),
            (Value::Int(n1), Value::BigInt(_)) => self.opcode_gt_inner(&int_to_bigint(*n1), v2),
            (Value::Int(n1), Value::Int(n2)) => {
                if n2 > n1 {
                    1
                } else {
                    0
                }
            }
            (Value::Int(n1), Value::Float(_)) => self.opcode_gt_inner(&int_to_float(*n1), v2),
            (Value::Float(_), Value::Int(n2)) => self.opcode_gt_inner(v1, &int_to_float(*n2)),
            (Value::Float(n1), Value::Float(n2)) => {
                if n2 > n1 {
                    1
                } else {
                    0
                }
            }
            (Value::DateTimeNT(d1), Value::DateTimeNT(d2)) => {
                if d2 > d1 {
                    1
                } else {
                    0
                }
            }
            (Value::DateTimeOT(d1), Value::DateTimeOT(d2)) => {
                if d2 > d1 {
                    1
                } else {
                    0
                }
            }
            (Value::DateTimeNT(d1), Value::DateTimeOT(d2)) => {
                if d2 > d1 {
                    1
                } else {
                    0
                }
            }
            (Value::DateTimeOT(d1), Value::DateTimeNT(d2)) => {
                if d2 > d1 {
                    1
                } else {
                    0
                }
            }
            (_, _) => {
                let n1_opt = v1.to_int();
                let n2_opt = v2.to_int();
                if let (Some(n1), Some(n2)) = (n1_opt, n2_opt) {
                    return if n2 > n1 { 1 } else { 0 };
                }
                let n1_opt = v1.to_bigint();
                let n2_opt = v2.to_bigint();
                if let (Some(n1), Some(n2)) = (n1_opt, n2_opt) {
                    return if n2 > n1 { 1 } else { 0 };
                }
                let n1_opt = v1.to_float();
                let n2_opt = v2.to_float();
                if let (Some(n1), Some(n2)) = (n1_opt, n2_opt) {
                    return if n2 > n1 { 1 } else { 0 };
                }

                let i1_str_opt: Option<&str>;
                to_str!(v1, i1_str_opt);

                let i2_str_opt: Option<&str>;
                to_str!(v2, i2_str_opt);

                if let (Some(n1), Some(n2)) = (i1_str_opt, i2_str_opt) {
                    return if n2 > n1 { 1 } else { 0 };
                }
                0
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
        if res == 1 {
            self.stack.push(Value::Bool(true));
        } else if res == 0 {
            self.stack.push(Value::Bool(false));
        } else {
            self.print_error("> requires two comparable values");
            return 0;
        }
        1
    }

    /// Helper function for checking whether one value is less than
    /// another.  Returns 1 if it is, 0 if it isn't, and -1 if the two
    /// values cannot be compared.
    pub fn opcode_lt_inner(&mut self, v1: &Value, v2: &Value) -> i32 {
        match (v1, v2) {
            (Value::BigInt(n1), Value::BigInt(n2)) => {
                if n2 < n1 {
                    1
                } else {
                    0
                }
            }
            (Value::BigInt(_), Value::Int(n2)) => self.opcode_lt_inner(v1, &int_to_bigint(*n2)),
            (Value::Int(n1), Value::BigInt(_)) => self.opcode_lt_inner(&int_to_bigint(*n1), v2),
            (Value::Int(n1), Value::Int(n2)) => {
                if n2 < n1 {
                    1
                } else {
                    0
                }
            }
            (Value::Int(n1), Value::Float(_)) => self.opcode_lt_inner(&int_to_float(*n1), v2),
            (Value::Float(_), Value::Int(n2)) => self.opcode_lt_inner(v1, &int_to_float(*n2)),
            (Value::Float(n1), Value::Float(n2)) => {
                if n2 < n1 {
                    1
                } else {
                    0
                }
            }
            (Value::DateTimeNT(d1), Value::DateTimeNT(d2)) => {
                if d2 < d1 {
                    1
                } else {
                    0
                }
            }
            (Value::DateTimeOT(d1), Value::DateTimeOT(d2)) => {
                if d2 < d1 {
                    1
                } else {
                    0
                }
            }
            (Value::DateTimeNT(d1), Value::DateTimeOT(d2)) => {
                if d2 < d1 {
                    1
                } else {
                    0
                }
            }
            (Value::DateTimeOT(d1), Value::DateTimeNT(d2)) => {
                if d2 < d1 {
                    1
                } else {
                    0
                }
            }
            (_, _) => {
                let n1_opt = v1.to_int();
                let n2_opt = v2.to_int();
                if let (Some(n1), Some(n2)) = (n1_opt, n2_opt) {
                    return if n2 < n1 { 1 } else { 0 };
                }
                let n1_opt = v1.to_bigint();
                let n2_opt = v2.to_bigint();
                if let (Some(n1), Some(n2)) = (n1_opt, n2_opt) {
                    return if n2 < n1 { 1 } else { 0 };
                }
                let n1_opt = v1.to_float();
                let n2_opt = v2.to_float();
                if let (Some(n1), Some(n2)) = (n1_opt, n2_opt) {
                    return if n2 < n1 { 1 } else { 0 };
                }

                let i1_str_opt: Option<&str>;
                to_str!(v1, i1_str_opt);

                let i2_str_opt: Option<&str>;
                to_str!(v2, i2_str_opt);

                if let (Some(n1), Some(n2)) = (i1_str_opt, i2_str_opt) {
                    return if n2 < n1 { 1 } else { 0 };
                }
                -1
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
        if res == 1 {
            self.stack.push(Value::Bool(true));
        } else if res == 0 {
            self.stack.push(Value::Bool(false));
        } else {
            self.print_error("< requires two comparable values");
            return 0;
        }
        1
    }

    /// Helper function for comparing two values.  Return 1 if the
    /// second value is greater than the first, 0 if the two values
    /// are equal, -1 if the second value is less than the first, and
    /// -2 if the two values cannot be compared.
    pub fn opcode_cmp_inner(&mut self, v1: &Value, v2: &Value) -> i32 {
        match (v1, v2) {
            (Value::BigInt(n1), Value::BigInt(n2)) => n2.cmp(n1) as i32,
            (Value::BigInt(_), Value::Int(n2)) => self.opcode_cmp_inner(v1, &int_to_bigint(*n2)),
            (Value::Int(n1), Value::BigInt(_)) => self.opcode_cmp_inner(&int_to_bigint(*n1), v2),
            (Value::Int(n1), Value::Int(n2)) => n2.cmp(n1) as i32,
            (Value::Int(n1), Value::Float(_)) => self.opcode_cmp_inner(&int_to_float(*n1), v2),
            (Value::Float(_), Value::Int(n2)) => self.opcode_cmp_inner(v1, &int_to_float(*n2)),
            (Value::Float(n1), Value::Float(n2)) => n2.partial_cmp(n1).unwrap() as i32,
            (Value::DateTimeNT(d1), Value::DateTimeNT(d2)) => d2.cmp(d1) as i32,
            (Value::DateTimeOT(d1), Value::DateTimeOT(d2)) => d2.cmp(d1) as i32,
            (Value::DateTimeNT(d1), Value::DateTimeOT(d2)) => {
                if d2 < d1 {
                    -1
                } else if d2 == d1 {
                    0
                } else {
                    -1
                }
            }
            (Value::DateTimeOT(d1), Value::DateTimeNT(d2)) => {
                if d2 < d1 {
                    -1
                } else if d2 == d1 {
                    0
                } else {
                    -1
                }
            }
            (_, _) => {
                let n1_opt = v1.to_int();
                let n2_opt = v2.to_int();
                if let (Some(n1), Some(n2)) = (n1_opt, n2_opt) {
                    return n2.cmp(&n1) as i32;
                }
                let n1_opt = v1.to_bigint();
                let n2_opt = v2.to_bigint();
                if let (Some(n1), Some(n2)) = (n1_opt, n2_opt) {
                    return n2.cmp(&n1) as i32;
                }
                let n1_opt = v1.to_float();
                let n2_opt = v2.to_float();
                if let (Some(n1), Some(n2)) = (n1_opt, n2_opt) {
                    return n2.partial_cmp(&n1).unwrap() as i32;
                }

                let i1_str_opt: Option<&str>;
                to_str!(v1, i1_str_opt);

                let i2_str_opt: Option<&str>;
                to_str!(v2, i2_str_opt);

                if let (Some(n1), Some(n2)) = (i1_str_opt, i2_str_opt) {
                    return n2.cmp(n1) as i32;
                }
                -2
            }
        }
    }

    /// Takes two values as its arguments, compares them, and places
    /// the result on the stack (-1 for less than, 0 for equal, and 1
    /// for greater than).
    pub fn opcode_cmp(&mut self) -> i32 {
        if self.stack.len() < 2 {
            self.print_error("<=> requires two arguments");
            return 0;
        }

        let v1_rr = self.stack.pop().unwrap();
        let v2_rr = self.stack.pop().unwrap();

        let res = self.opcode_cmp_inner(&v1_rr, &v2_rr);
        if res == 1 || res == 0 || res == -1 {
            self.stack.push(Value::Int(res));
        } else {
            self.print_error("<=> requires two comparable values");
            return 0;
        }
        1
    }

    /// Get the square root of a number.
    pub fn core_sqrt(&mut self) -> i32 {
        if self.stack.is_empty() {
            self.print_error("sqrt requires one argument");
            return 0;
        }

        let value_rr = self.stack.pop().unwrap();
        let f_opt = value_rr.to_float();
        match f_opt {
            Some(f) => {
                let fs = f.sqrt();
                self.stack.push(Value::Float(fs));
                1
            }
            None => {
                self.print_error("sqrt argument must be float");
                0
            }
        }
    }

    /// Helper function for exponentiation.
    fn core_exp_inner(&mut self, v1: &Value, v2: &Value) -> i32 {
        match (v1, v2) {
            (Value::Int(n), Value::Int(exp)) => {
                if *exp < 0 {
                    self.print_error("second exp argument cannot be negative");
                    return 0;
                }
                let nn = (*n).checked_pow((*exp).try_into().unwrap());
                match nn {
                    Some(nnn) => {
                        self.stack.push(Value::Int(nnn));
                        1
                    }
                    None => {
                        let bi = BigInt::from_i32(*n).unwrap();
                        let bb = bi.pow((*exp).try_into().unwrap());
                        self.stack.push(Value::BigInt(bb));
                        1
                    }
                }
            }
            (Value::Float(f), Value::Int(exp)) => {
                if *exp < 0 {
                    self.print_error("second exp argument cannot be negative");
                    return 0;
                }
                let ff = (*f).powf((*exp).try_into().unwrap());
                self.stack.push(Value::Float(ff));
                1
            }
            (Value::BigInt(bi), Value::Int(exp)) => {
                if *exp < 0 {
                    self.print_error("second exp argument cannot be negative");
                    return 0;
                }
                let bb = (*bi).pow((*exp).try_into().unwrap());
                self.stack.push(Value::BigInt(bb));
                1
            }
            (Value::Int(n), Value::Float(exp)) => {
                if *exp < 0.0 {
                    self.print_error("second exp argument cannot be negative");
                    return 0;
                }
                let f = *n as f64;
                let ff = f.powf(*exp);
                self.stack.push(Value::Float(ff));
                1
            }
            (Value::Float(f), Value::Float(exp)) => {
                if *exp < 0.0 {
                    self.print_error("second exp argument cannot be negative");
                    return 0;
                }
                let ff = (*f).powf(*exp);
                self.stack.push(Value::Float(ff));
                1
            }
            (Value::BigInt(bi), Value::Float(exp)) => {
                if *exp < 0.0 {
                    self.print_error("second exp argument cannot be negative");
                    return 0;
                }
                let ff = (*bi).to_f64().unwrap().powf(*exp);
                self.stack.push(Value::Float(ff));
                1
            }
            (Value::Int(_), _) => {
                let n_opt = v2.to_int();
                if let Some(n) = n_opt {
                    return self.core_exp_inner(v1, &Value::Int(n));
                }

                let f_opt = v2.to_float();
                if let Some(f) = f_opt {
                    return self.core_exp_inner(v1, &Value::Float(f));
                }

                0
            }
            (Value::BigInt(_), _) => {
                let n_opt = v2.to_int();
                if let Some(n) = n_opt {
                    return self.core_exp_inner(v1, &Value::Int(n));
                }

                let f_opt = v2.to_float();
                if let Some(f) = f_opt {
                    return self.core_exp_inner(v1, &Value::Float(f));
                }

                0
            }
            (Value::Float(_), _) => {
                let n_opt = v2.to_int();
                if let Some(n) = n_opt {
                    return self.core_exp_inner(v1, &Value::Int(n));
                }

                let f_opt = v2.to_float();
                if let Some(f) = f_opt {
                    return self.core_exp_inner(v1, &Value::Float(f));
                }

                0
            }
            (_, _) => {
                let n_opt = v1.to_int();
                if let Some(n) = n_opt {
                    return self.core_exp_inner(&Value::Int(n), v2);
                }

                let f_opt = v1.to_float();
                if let Some(f) = f_opt {
                    return self.core_exp_inner(&Value::Float(f), v2);
                }

                let bi_opt = v1.to_bigint();
                if let Some(bi) = bi_opt {
                    return self.core_exp_inner(&Value::BigInt(bi), v2);
                }

                0
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
            self.print_error("exp arguments unable to be handled");
            return 0;
        }

        1
    }

    /// Get the absolute value of the argument.
    pub fn core_abs(&mut self) -> i32 {
        if self.stack.is_empty() {
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
        if let Some(n) = n_opt {
            let nn = n.abs();
            self.stack.push(Value::Int(nn));
            return 1;
        }

        let bi_opt = value_rr.to_bigint();
        if let Some(bi) = bi_opt {
            let bb = bi.abs();
            self.stack.push(Value::BigInt(bb));
            return 1;
        }

        let f_opt = value_rr.to_float();
        if let Some(f) = f_opt {
            let ff = f.abs();
            self.stack.push(Value::Float(ff));
            return 1;
        }

        self.print_error("abs argument unable to be handled");
        0
    }

    /// Helper function for left shift.
    fn core_lsft_inner(&mut self, v1: &Value, v2: &Value) -> i32 {
        match (v1, v2) {
            (Value::Int(n), Value::Int(shift)) => {
                if *n < 0 {
                    return 0;
                }
                let un: u32 = (*n).try_into().unwrap();
                /* One fewer zero, to account for the conversion from
                 * unsigned to signed. */
                let zeros = un.leading_zeros() as i32 - 1;
                if *shift > zeros {
                    let bi = BigInt::from_u32(un).unwrap();
                    let bb = bi << shift;
                    self.stack.push(Value::BigInt(bb));
                    return 1;
                }
                let nn = un << shift;
                self.stack.push(Value::Int(nn.try_into().unwrap()));
                1
            }
            (Value::BigInt(bi), Value::Int(shift)) => {
                let bb = bi << shift;
                self.stack.push(Value::BigInt(bb));
                1
            }
            (Value::Int(_), _) => {
                let n_opt = v2.to_int();
                if let Some(n) = n_opt {
                    return self.core_lsft_inner(v1, &Value::Int(n));
                }
                0
            }
            (Value::BigInt(_), _) => {
                let n_opt = v2.to_int();
                if let Some(n) = n_opt {
                    return self.core_lsft_inner(v1, &Value::Int(n));
                }
                0
            }
            (_, _) => {
                let n_opt = v1.to_int();
                if let Some(n) = n_opt {
                    return self.core_lsft_inner(&Value::Int(n), v2);
                }
                let bi_opt = v1.to_bigint();
                if let Some(bi) = bi_opt {
                    return self.core_lsft_inner(&Value::BigInt(bi), v2);
                }
                0
            }
        }
    }

    /// Shift the first argument left the specified number of times.
    pub fn core_lsft(&mut self) -> i32 {
        if self.stack.len() < 2 {
            self.print_error("<< requires two arguments");
            return 0;
        }

        let shift_rr = self.stack.pop().unwrap();
        let value_rr = self.stack.pop().unwrap();

        let res = self.core_lsft_inner(&value_rr, &shift_rr);
        if res == 0 {
            self.print_error("<< arguments unable to be handled");
            return 0;
        }

        1
    }

    /// Helper function for right shift.
    fn core_rsft_inner(&mut self, v1: &Value, v2: &Value) -> i32 {
        match (v1, v2) {
            (Value::Int(n), Value::Int(shift)) => {
                if *n < 0 {
                    return 0;
                }
                let un: u32 = (*n).try_into().unwrap();
                let nn = un >> shift;
                self.stack.push(Value::Int(nn.try_into().unwrap()));
                1
            }
            (Value::BigInt(bi), Value::Int(shift)) => {
                let bb = bi >> shift;
                self.stack.push(Value::BigInt(bb));
                1
            }
            (Value::Int(_), _) => {
                let n_opt = v2.to_int();
                if let Some(n) = n_opt {
                    return self.core_rsft_inner(v1, &Value::Int(n));
                }
                0
            }
            (Value::BigInt(_), _) => {
                let n_opt = v2.to_int();
                if let Some(n) = n_opt {
                    return self.core_rsft_inner(v1, &Value::Int(n));
                }
                0
            }
            (_, _) => {
                let n_opt = v1.to_int();
                if let Some(n) = n_opt {
                    return self.core_rsft_inner(&Value::Int(n), v2);
                }
                let bi_opt = v1.to_bigint();
                if let Some(bi) = bi_opt {
                    return self.core_rsft_inner(&Value::BigInt(bi), v2);
                }
                0
            }
        }
    }

    /// Shift the first argument right the specified number of times.
    pub fn core_rsft(&mut self) -> i32 {
        if self.stack.len() < 2 {
            self.print_error(">> requires two arguments");
            return 0;
        }

        let shift_rr = self.stack.pop().unwrap();
        let value_rr = self.stack.pop().unwrap();

        let res = self.core_rsft_inner(&value_rr, &shift_rr);
        if res == 0 {
            self.print_error(">> arguments unable to be handled");
            return 0;
        }

        1
    }

    /// Helper function for bitwise xor.
    fn core_xor_inner(&mut self, v1: &Value, v2: &Value) -> i32 {
        match (v1, v2) {
            (Value::Int(left), Value::Int(right)) => {
                if *left < 0 {
                    return 0;
                }
                if *right < 0 {
                    return 0;
                }
                let u_left: u32 = (*left).try_into().unwrap();
                let u_right: u32 = (*right).try_into().unwrap();
                let result = u_left ^ u_right;
                let int_result: Result<i32, _> = result.try_into();
                match int_result {
                    Ok(n) => {
                        self.stack.push(Value::Int(n));
                    }
                    Err(_) => {
                        self.stack
                            .push(Value::BigInt(BigInt::from_u32(result).unwrap()));
                    }
                }
                1
            }
            (Value::BigInt(left), Value::Int(right)) => {
                let result = left ^ BigInt::from_i32(*right).unwrap();
                self.stack.push(Value::BigInt(result));
                1
            }
            (Value::BigInt(left), Value::BigInt(right)) => {
                let result = left ^ right;
                self.stack.push(Value::BigInt(result));
                1
            }
            (Value::Int(_), _) => {
                let n_opt = v2.to_int();
                if let Some(n) = n_opt {
                    return self.core_xor_inner(v1, &Value::Int(n));
                }
                0
            }
            (Value::BigInt(_), _) => {
                let n_opt = v2.to_int();
                if let Some(n) = n_opt {
                    return self.core_xor_inner(v1, &Value::Int(n));
                }
                let bi_opt = v2.to_bigint();
                if let Some(bi) = bi_opt {
                    return self.core_xor_inner(v1, &Value::BigInt(bi));
                }
                0
            }
            (_, _) => {
                let n_opt = v1.to_int();
                if let Some(n) = n_opt {
                    return self.core_xor_inner(&Value::Int(n), v2);
                }
                let bi_opt = v1.to_bigint();
                if let Some(bi) = bi_opt {
                    return self.core_xor_inner(&Value::BigInt(bi), v2);
                }
                0
            }
        }
    }

    /// Perform a bitwise xor on the arguments.
    pub fn core_xor(&mut self) -> i32 {
        if self.stack.len() < 2 {
            self.print_error("^ requires two arguments");
            return 0;
        }

        let left_rr = self.stack.pop().unwrap();
        let right_rr = self.stack.pop().unwrap();

        let res = self.core_xor_inner(&left_rr, &right_rr);
        if res == 0 {
            self.print_error("^ arguments unable to be handled");
            return 0;
        }

        1
    }

    /// Helper function for bitwise or.
    fn core_or_inner(&mut self, v1: &Value, v2: &Value) -> i32 {
        match (v1, v2) {
            (Value::Int(left), Value::Int(right)) => {
                if *left < 0 {
                    return 0;
                }
                if *right < 0 {
                    return 0;
                }
                let u_left: u32 = (*left).try_into().unwrap();
                let u_right: u32 = (*right).try_into().unwrap();
                let result = u_left | u_right;
                let int_result: Result<i32, _> = result.try_into();
                match int_result {
                    Ok(n) => {
                        self.stack.push(Value::Int(n));
                    }
                    Err(_) => {
                        self.stack
                            .push(Value::BigInt(BigInt::from_u32(result).unwrap()));
                    }
                }
                1
            }
            (Value::BigInt(left), Value::Int(right)) => {
                let result = left | BigInt::from_i32(*right).unwrap();
                self.stack.push(Value::BigInt(result));
                1
            }
            (Value::BigInt(left), Value::BigInt(right)) => {
                let result = left | right;
                self.stack.push(Value::BigInt(result));
                1
            }
            (Value::Int(_), _) => {
                let n_opt = v2.to_int();
                if let Some(n) = n_opt {
                    return self.core_or_inner(v1, &Value::Int(n));
                }
                0
            }
            (Value::BigInt(_), _) => {
                let n_opt = v2.to_int();
                if let Some(n) = n_opt {
                    return self.core_or_inner(v1, &Value::Int(n));
                }
                let bi_opt = v2.to_bigint();
                if let Some(bi) = bi_opt {
                    return self.core_or_inner(v1, &Value::BigInt(bi));
                }
                0
            }
            (_, _) => {
                let n_opt = v1.to_int();
                if let Some(n) = n_opt {
                    return self.core_or_inner(&Value::Int(n), v2);
                }
                let bi_opt = v1.to_bigint();
                if let Some(bi) = bi_opt {
                    return self.core_or_inner(&Value::BigInt(bi), v2);
                }
                0
            }
        }
    }

    /// Perform a bitwise or on the arguments.
    pub fn core_or(&mut self) -> i32 {
        if self.stack.len() < 2 {
            self.print_error("|| requires two arguments");
            return 0;
        }

        let left_rr = self.stack.pop().unwrap();
        let right_rr = self.stack.pop().unwrap();

        let res = self.core_or_inner(&left_rr, &right_rr);
        if res == 0 {
            self.print_error("|| arguments unable to be handled");
            return 0;
        }

        1
    }

    /// Helper function for bitwise and.
    fn core_and_inner(&mut self, v1: &Value, v2: &Value) -> i32 {
        match (v1, v2) {
            (Value::Int(left), Value::Int(right)) => {
                if *left < 0 {
                    return 0;
                }
                if *right < 0 {
                    return 0;
                }
                let u_left: u32 = (*left).try_into().unwrap();
                let u_right: u32 = (*right).try_into().unwrap();
                let result = u_left & u_right;
                let int_result: Result<i32, _> = result.try_into();
                match int_result {
                    Ok(n) => {
                        self.stack.push(Value::Int(n));
                    }
                    Err(_) => {
                        self.stack
                            .push(Value::BigInt(BigInt::from_u32(result).unwrap()));
                    }
                }
                1
            }
            (Value::BigInt(left), Value::Int(right)) => {
                let result = left & BigInt::from_i32(*right).unwrap();
                self.stack.push(Value::BigInt(result));
                1
            }
            (Value::BigInt(left), Value::BigInt(right)) => {
                let result = left & right;
                self.stack.push(Value::BigInt(result));
                1
            }
            (Value::Int(_), _) => {
                let n_opt = v2.to_int();
                if let Some(n) = n_opt {
                    return self.core_and_inner(v1, &Value::Int(n));
                }
                0
            }
            (Value::BigInt(_), _) => {
                let n_opt = v2.to_int();
                if let Some(n) = n_opt {
                    return self.core_and_inner(v1, &Value::Int(n));
                }
                let bi_opt = v2.to_bigint();
                if let Some(bi) = bi_opt {
                    return self.core_and_inner(v1, &Value::BigInt(bi));
                }
                0
            }
            (_, _) => {
                let n_opt = v1.to_int();
                if let Some(n) = n_opt {
                    return self.core_and_inner(&Value::Int(n), v2);
                }
                let bi_opt = v1.to_bigint();
                if let Some(bi) = bi_opt {
                    return self.core_and_inner(&Value::BigInt(bi), v2);
                }
                0
            }
        }
    }

    /// Perform a bitwise and on the arguments.
    pub fn core_and(&mut self) -> i32 {
        if self.stack.len() < 2 {
            self.print_error("& requires two arguments");
            return 0;
        }

        let left_rr = self.stack.pop().unwrap();
        let right_rr = self.stack.pop().unwrap();

        let res = self.core_and_inner(&left_rr, &right_rr);
        if res == 0 {
            self.print_error("& arguments unable to be handled");
            return 0;
        }

        1
    }
}
