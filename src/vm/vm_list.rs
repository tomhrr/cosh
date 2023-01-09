use std::cell::RefCell;
use std::collections::VecDeque;
use std::mem;
use std::rc::Rc;

use indexmap::IndexMap;
use ipnet::{Ipv4Net, Ipv6Net};
use iprange::IpRange;

use chunk::{IpSet, StringTriple, Value};
use vm::VM;

impl VM {
    /// Takes a list and an index as its arguments.  Gets the element
    /// at the given index from the list and places it onto the stack.
    pub fn core_nth(&mut self) -> i32 {
        if self.stack.len() < 2 {
            self.print_error("nth requires two arguments");
            return 0;
        }

        let index_rr = self.stack.pop().unwrap();
        let index_int_opt = index_rr.to_int();

        let lst_rr = self.stack.pop().unwrap();

        match (index_int_opt, &lst_rr) {
            (Some(index), Value::List(lst)) => {
                if lst.borrow().len() <= (index as usize) {
                    self.print_error("second nth argument must fall within list bounds");
                    return 0;
                }
                let element = lst.borrow()[index as usize].clone();
                self.stack.push(element);
                return 1;
            }
            (_, _) => {}
        }

        self.stack.push(lst_rr);
        match index_int_opt {
            Some(mut index) => {
                while index >= 0 {
                    let dup_res = self.opcode_dup();
                    if dup_res == 0 {
                        return 0;
                    }
                    let shift_res = self.opcode_shift();
                    if shift_res == 0 {
                        return 0;
                    }
                    if index == 0 {
                        self.stack.remove(self.stack.len() - 2);
                        break;
                    } else {
                        self.stack.pop();
                        index = index - 1;
                    }
                }
                return 1;
            }
            _ => {
                self.print_error("second nth argument must be integer");
                return 0;
            }
        }
    }

    /// Takes a list, an index, and a value as its arguments.  Places
    /// the value at the given index in the list.
    pub fn core_nth_em(&mut self) -> i32 {
        if self.stack.len() < 3 {
            self.print_error("nth! requires three arguments");
            return 0;
        }

        let val_rr = self.stack.pop().unwrap();

        let index_rr = self.stack.pop().unwrap();
        let index_int_opt = index_rr.to_int();

        let mut lst_rr = self.stack.pop().unwrap();

        {
            match (index_int_opt, &mut lst_rr) {
                (Some(index), Value::List(lst)) => {
                    if lst.borrow().len() <= (index as usize) {
                        self.print_error("second nth! argument must fall within list bounds");
                        return 0;
                    }
                    lst.borrow_mut()[index as usize] = val_rr;
                }
                (Some(_), _) => {
                    self.print_error("first nth! argument must be list");
                    return 0;
                }
                (_, _) => {
                    self.print_error("second nth! argument must be integer");
                    return 0;
                }
            }
        }
        self.stack.push(lst_rr);
        return 1;
    }

    /// Takes a list or a set and a value as its arguments.  Pushes
    /// the value onto the list/set and places the updated list/set
    /// onto the stack.
    pub fn opcode_push(&mut self) -> i32 {
        if self.stack.len() < 2 {
            self.print_error("push requires two arguments");
            return 0;
        }

        let element_rr = self.stack.pop().unwrap();
        let mut lst_rr = self.stack.pop().unwrap();

        {
            match lst_rr {
                Value::List(ref mut lst) => {
                    lst.borrow_mut().push_back(element_rr);
                }
                Value::Set(ref mut map) => {
                    {
                        let mb = map.borrow();
                        if !mb.is_empty() {
                            let (_, val) = mb.iter().next().unwrap().clone();
                            if !val.variants_equal(&element_rr) {
                                self.print_error(
                                    "second push argument type does not match first argument set",
                                );
                                return 0;
                            }
                        }
                    }

                    /* Disallow set creation for IP
                     * addresses or IP sets: users should
                     * just use IP sets in those cases. */
                    match element_rr {
                        Value::IpSet(_)
                        | Value::Ipv4(_)
                        | Value::Ipv6(_)
                        | Value::Ipv4Range(_)
                        | Value::Ipv6Range(_) => {
                            self.print_error(
                                "second push argument cannot be an IP address object (see ips)",
                            );
                            return 0;
                        }
                        _ => {}
                    }

                    let element_str_opt: Option<&str>;
                    to_str!(element_rr.clone(), element_str_opt);
                    match element_str_opt {
                        None => {
                            self.print_error("second push argument cannot be added to set");
                            return 0;
                        }
                        Some(s) => {
                            map.borrow_mut().insert(s.to_string(), element_rr);
                        }
                    }
                }
                _ => {
                    self.print_error("first push argument must be list/set");
                    return 0;
                }
            }
        }

        self.stack.push(lst_rr);
        return 1;
    }

    /// Takes a list and a value as its arguments.  Pushes the value
    /// onto the start of the list and places the updated list onto
    /// the stack.
    pub fn core_unshift(&mut self) -> i32 {
        if self.stack.len() < 2 {
            self.print_error("unshift requires two arguments");
            return 0;
        }

        let element_rr = self.stack.pop().unwrap();
        let mut lst_rr = self.stack.pop().unwrap();

        {
            match lst_rr {
                Value::List(ref mut lst) => {
                    lst.borrow_mut().push_front(element_rr);
                }
                _ => {
                    self.print_error("first unshift argument must be list");
                    return 0;
                }
            }
        }

        self.stack.push(lst_rr);
        return 1;
    }

    /// Takes a list as its single argument.  Pops a value from the
    /// end of the list and places that value onto the stack.
    pub fn opcode_pop(&mut self) -> i32 {
        if self.stack.is_empty() {
            self.print_error("pop requires one argument");
            return 0;
        }

        let mut lst_rr = self.stack.pop().unwrap();
        let element_rr = match lst_rr {
            Value::List(ref mut lst) => {
                let element_rr_opt = lst.borrow_mut().pop_back();
                match element_rr_opt {
                    Some(element_rr) => element_rr,
                    None => Value::Null,
                }
            }
            _ => {
                self.print_error("pop argument must be list");
                return 0;
            }
        };

        self.stack.push(element_rr);
        return 1;
    }

    pub fn opcode_shift_inner<'a>(&mut self, shiftable_rr: &mut Value) -> i32 {
        let mut repush = false;
        let mut stack_len = 0;
        let mut new_stack_len = 0;

        match *shiftable_rr {
            Value::Generator(ref mut generator_object_) => {
                // todo: set to none, error later if still none.
                let mut new_i = 0;
                {
                    let mut generator_object = generator_object_.borrow_mut();
                    let index = generator_object.index;

                    {
                        let gen_args = &mut generator_object.gen_args;
                        stack_len = self.stack.len();
                        let mut is_empty = false;
                        if gen_args.len() == 1 {
                            let gen_arg_rr = &gen_args[0];
                            match gen_arg_rr {
                                Value::Null => {
                                    is_empty = true;
                                }
                                _ => {
                                    is_empty = false;
                                }
                            }
                        }
                        if is_empty {
                            gen_args.pop();
                            self.stack.push(Value::Int(0));
                        } else {
                            let gen_args_len = gen_args.len();
                            if gen_args_len > 0 {
                                while gen_args.len() > 0 {
                                    self.stack.push(gen_args.pop().unwrap());
                                }
                                self.stack.push(Value::Int(gen_args_len as i32));
                            }
                        }
                    }

                    /* todo: need a generator-specific run function,
                     * to avoid the stuffing around here. */
                    let local_vars_stack = generator_object.local_vars_stack.clone();
                    let chunk = generator_object.chunk.clone();
                    let call_stack_chunks = &mut generator_object.call_stack_chunks;
                    mem::swap(call_stack_chunks, &mut self.call_stack_chunks);

                    let current_index = index;
                    if current_index == chunk.borrow().data.len() {
                        /* At end of function: push null. */
                        self.stack.push(Value::Null);
                    } else {
                        let plvs_stack = self.local_var_stack.clone();
                        self.local_var_stack = local_vars_stack;
                        let backup_chunk = self.chunk.clone();
                        self.chunk = chunk.clone();
                        let i = self.i;
                        self.i = index;
                        let res = self.run_inner();
                        self.i = i;
                        self.chunk = backup_chunk;
                        self.local_var_stack = plvs_stack;
                        mem::swap(call_stack_chunks, &mut self.call_stack_chunks);
                        match res {
                            0 => {
                                return 0;
                            }
                            i => {
                                new_i = i;
                                new_stack_len = self.stack.len();
                                repush = true;
                            }
                        }
                    }
                }
                generator_object_.borrow_mut().index = new_i;
            }
            Value::List(ref mut lst) => {
                if lst.borrow().len() > 0 {
                    let value_rr = lst.borrow_mut().pop_front().unwrap();
                    self.stack.push(value_rr);
                } else {
                    self.stack.push(Value::Null);
                }
            }
            Value::Set(ref mut hsh) => {
                if hsh.borrow().len() > 0 {
                    /* shift_remove_index takes O(n), which is
                     * unpleasant, but necessary for uniformity with
                     * how lists are processed.  There is probably a
                     * more appropriate structure that can be used for
                     * sets. */
                    let (_, value_rr) = hsh.borrow_mut().shift_remove_index(0).unwrap();
                    self.stack.push(value_rr);
                } else {
                    self.stack.push(Value::Null);
                }
            }
            Value::IpSet(ref mut ipset) => {
                /* todo: not sure how else to implement this, since
                 * the IP range object doesn't order by address.
                 * Could serialise to a vector in the IPSet object,
                 * but that might make other operations less
                 * efficient. */
                let mut ipranges = ipset.borrow().ipv4.iter().collect::<Vec<Ipv4Net>>();
                ipranges.sort_by(|a, b| a.network().cmp(&b.network()));
                let next = ipranges.iter().next();
                if !next.is_none() {
                    let next_value = next.unwrap();
                    let mut next_range = IpRange::new();
                    next_range.add(*next_value);
                    let new_set = ipset.borrow().ipv4.exclude(&next_range);
                    ipset.borrow_mut().ipv4 = new_set;
                    self.stack.push(Value::Ipv4(*next_value));
                    return 1;
                }
                let mut ipranges2 = ipset.borrow().ipv6.iter().collect::<Vec<Ipv6Net>>();
                ipranges2.sort_by(|a, b| a.network().cmp(&b.network()));
                let next2 = ipranges2.iter().next();
                if !next2.is_none() {
                    let next2_value = next2.unwrap();
                    let mut next2_range = IpRange::new();
                    next2_range.add(*next2_value);
                    let new_set = ipset.borrow().ipv6.exclude(&next2_range);
                    ipset.borrow_mut().ipv6 = new_set;
                    self.stack.push(Value::Ipv6(*next2_value));
                    return 1;
                }
                self.stack.push(Value::Null);
                return 1;
            }
            Value::CommandGenerator(ref mut command_generator) => {
                let mut cg = command_generator.borrow_mut();
                if cg.get_combined {
                    let str_opt = cg.read_line_combined();
                    match str_opt {
                        None => {
                            self.stack.push(Value::Null);
                        }
                        Some((i, s)) => {
                            let mut lst = VecDeque::new();
                            lst.push_back(Value::Int(i));
                            lst.push_back(Value::String(Rc::new(RefCell::new(StringTriple::new(
                                s, None,
                            )))));
                            self.stack.push(Value::List(Rc::new(RefCell::new(lst))));
                        }
                    }
                } else {
                    let str_opt = cg.read_line();
                    match str_opt {
                        None => {
                            self.stack.push(Value::Null);
                        }
                        Some(s) => {
                            self.stack.push(Value::String(Rc::new(RefCell::new(
                                StringTriple::new(s, None),
                            ))));
                        }
                    }
                }
                return 1;
            }
            Value::KeysGenerator(ref mut hwi) => {
                {
                    let hash_rr = &hwi.borrow().h;
                    match hash_rr {
                        Value::Hash(map) => {
                            let mapb = map.borrow();
                            let kv = mapb.get_index(hwi.borrow().i);
                            match kv {
                                Some((k, _)) => {
                                    self.stack.push(Value::String(Rc::new(RefCell::new(
                                        StringTriple::new(k.to_string(), None),
                                    ))));
                                }
                                None => {
                                    self.stack.push(Value::Null);
                                }
                            }
                        }
                        _ => {
                            eprintln!("keys generator does not contain a hash!");
                            std::process::abort();
                        }
                    }
                }
                let el = hwi.borrow().i + 1;
                hwi.borrow_mut().i = el;
            }
            Value::ValuesGenerator(ref mut hwi) => {
                {
                    let hash_rr = &hwi.borrow().h;
                    match hash_rr {
                        Value::Hash(map) => {
                            let mapb = map.borrow();
                            let kv = mapb.get_index(hwi.borrow().i);
                            match kv {
                                Some((_, v)) => {
                                    self.stack.push(v.clone());
                                }
                                None => {
                                    self.stack.push(Value::Null);
                                }
                            }
                        }
                        _ => {
                            eprintln!("values generator does not contain a hash!");
                            std::process::abort();
                        }
                    }
                }
                let el = hwi.borrow().i + 1;
                hwi.borrow_mut().i = el;
            }
            Value::EachGenerator(ref mut hwi) => {
                {
                    let hash_rr = &hwi.borrow().h;
                    match hash_rr {
                        Value::Hash(map) => {
                            let mapb = map.borrow();
                            let kv = mapb.get_index(hwi.borrow().i);
                            match kv {
                                Some((k, v)) => {
                                    let mut lst = VecDeque::new();
                                    lst.push_back(Value::String(Rc::new(RefCell::new(
                                        StringTriple::new(k.to_string(), None),
                                    ))));
                                    lst.push_back(v.clone());
                                    self.stack.push(Value::List(Rc::new(RefCell::new(lst))));
                                }
                                None => {
                                    self.stack.push(Value::Null);
                                }
                            }
                        }
                        _ => {
                            eprintln!("each generator does not contain a hash!");
                            std::process::abort();
                        }
                    }
                }
                let el = hwi.borrow().i + 1;
                hwi.borrow_mut().i = el;
            }
            Value::MultiGenerator(ref mut genlist_rr) => {
                let mut genlist = genlist_rr.borrow_mut();
                loop {
                    if genlist.len() == 0 {
                        self.stack.push(Value::Null);
                        break;
                    } else {
                        let mut next = genlist.front_mut().unwrap();
                        self.opcode_shift_inner(&mut next);
                        if self.stack.len() == 0 {
                            return 0;
                        }
                        match self.stack[self.stack.len() - 1] {
                            Value::Null => {
                                self.stack.pop();
                                genlist.pop_front();
                                continue;
                            }
                            _ => {
                                break;
                            }
                        }
                    }
                }
            }
            _ => {
                self.print_error("shift argument does not support shift");
                return 0;
            }
        }
        if repush && new_stack_len == stack_len {
            self.stack.push(Value::Null);
        }

        return 1;
    }

    /// Takes a shiftable object as its single argument.  Shifts an
    /// element from that object and puts it onto the stack.
    pub fn opcode_shift<'a>(&mut self) -> i32 {
        if self.stack.is_empty() {
            self.print_error("shift requires one argument");
            return 0;
        }

        let mut shiftable_rr = self.stack.pop().unwrap();
        return self.opcode_shift_inner(&mut shiftable_rr);
    }

    /// Takes an arbitrary value as its single argument.  Places a
    /// boolean onto the stack indicating whether the argument can be
    /// shifted.
    pub fn opcode_isshiftable(&mut self) -> i32 {
        if self.stack.is_empty() {
            self.print_error("is-shiftable requires one argument");
            return 0;
        }

        let value_rr = self.stack.pop().unwrap();
        let res = match value_rr {
            Value::List(_) => true,
            Value::Set(_) => true,
            Value::IpSet(_) => true,
            Value::Generator(_) => true,
            Value::CommandGenerator(_) => true,
            Value::KeysGenerator(_) => true,
            Value::ValuesGenerator(_) => true,
            Value::EachGenerator(_) => true,
            _ => false,
        };
        self.stack.push(Value::Bool(res));
        return 1;
    }

    /// Takes two sets as its arguments and returns their union.
    pub fn core_union(&mut self) -> i32 {
        if self.stack.len() < 2 {
            self.print_error("union requires two arguments");
            return 0;
        }

        let set2_rr = self.stack.pop().unwrap();
        let set1_rr = self.stack.pop().unwrap();

        match (set1_rr, set2_rr) {
            (Value::Set(s1), Value::Set(s2)) => {
                let mut new_hsh = IndexMap::new();
                for (k, v) in s1.borrow().iter() {
                    new_hsh.insert(k.clone(), v.value_clone());
                }
                for (k, v) in s2.borrow().iter() {
                    new_hsh.insert(k.clone(), v.value_clone());
                }
                let set = Value::Set(Rc::new(RefCell::new(new_hsh)));
                self.stack.push(set);
            }
            (Value::IpSet(ipset1), Value::IpSet(ipset2)) => {
                let ipset1_ipv4 = &ipset1.borrow().ipv4;
                let ipset1_ipv6 = &ipset1.borrow().ipv6;
                let ipset2_ipv4 = &ipset2.borrow().ipv4;
                let ipset2_ipv6 = &ipset2.borrow().ipv6;
                let new_ipv4 = ipset1_ipv4.merge(&ipset2_ipv4);
                let new_ipv6 = ipset1_ipv6.merge(&ipset2_ipv6);
                let new_ipset = IpSet::new(new_ipv4, new_ipv6);
                self.stack
                    .push(Value::IpSet(Rc::new(RefCell::new(new_ipset))));
                return 1;
            }
            (Value::Set(_), _) => {
                self.print_error("second union argument must be set");
                return 0;
            }
            (_, _) => {
                self.print_error("first union argument must be set");
                return 0;
            }
        }
        return 1;
    }

    /// Takes two sets as its arguments and returns their
    /// intersection.
    pub fn core_isect(&mut self) -> i32 {
        if self.stack.len() < 2 {
            self.print_error("isect requires two arguments");
            return 0;
        }

        let set2_rr = self.stack.pop().unwrap();
        let set1_rr = self.stack.pop().unwrap();

        match (set1_rr, set2_rr) {
            (Value::Set(s1), Value::Set(s2)) => {
                let mut new_hsh = IndexMap::new();
                for (k, v) in s1.borrow().iter() {
                    if !s2.borrow().get(k).is_none() {
                        new_hsh.insert(k.clone(), v.value_clone());
                    }
                }
                let set = Value::Set(Rc::new(RefCell::new(new_hsh)));
                self.stack.push(set);
            }
            (Value::IpSet(ipset1), Value::IpSet(ipset2)) => {
                let ipset1_ipv4 = &ipset1.borrow().ipv4;
                let ipset1_ipv6 = &ipset1.borrow().ipv6;
                let ipset2_ipv4 = &ipset2.borrow().ipv4;
                let ipset2_ipv6 = &ipset2.borrow().ipv6;
                let new_ipv4 = ipset1_ipv4.intersect(&ipset2_ipv4);
                let new_ipv6 = ipset1_ipv6.intersect(&ipset2_ipv6);
                let new_ipset = IpSet::new(new_ipv4, new_ipv6);
                self.stack
                    .push(Value::IpSet(Rc::new(RefCell::new(new_ipset))));
                return 1;
            }
            (Value::Set(_), _) => {
                self.print_error("second isect argument must be set");
                return 0;
            }
            (_, _) => {
                self.print_error("first isect argument must be set");
                return 0;
            }
        }
        return 1;
    }

    /// Takes two sets as its arguments and returns their
    /// difference.
    pub fn core_diff(&mut self) -> i32 {
        if self.stack.len() < 2 {
            self.print_error("diff requires two arguments");
            return 0;
        }

        let set2_rr = self.stack.pop().unwrap();
        let set1_rr = self.stack.pop().unwrap();

        match (set1_rr, set2_rr) {
            (Value::Set(s1), Value::Set(s2)) => {
                let mut new_hsh = IndexMap::new();
                for (k, v) in s1.borrow().iter() {
                    if s2.borrow().get(k).is_none() {
                        new_hsh.insert(k.clone(), v.value_clone());
                    }
                }
                let set = Value::Set(Rc::new(RefCell::new(new_hsh)));
                self.stack.push(set);
            }
            (Value::IpSet(ipset1), Value::IpSet(ipset2)) => {
                let ipset1_ipv4 = &ipset1.borrow().ipv4;
                let ipset1_ipv6 = &ipset1.borrow().ipv6;
                let ipset2_ipv4 = &ipset2.borrow().ipv4;
                let ipset2_ipv6 = &ipset2.borrow().ipv6;
                let new_ipv4 = ipset1_ipv4.exclude(&ipset2_ipv4);
                let new_ipv6 = ipset1_ipv6.exclude(&ipset2_ipv6);
                let new_ipset = IpSet::new(new_ipv4, new_ipv6);
                self.stack
                    .push(Value::IpSet(Rc::new(RefCell::new(new_ipset))));
                return 1;
            }
            (Value::Set(_), _) => {
                self.print_error("second diff argument must be set");
                return 0;
            }
            (_, _) => {
                self.print_error("first diff argument must be set");
                return 0;
            }
        }
        return 1;
    }

    /// Takes two sets as its arguments and returns their
    /// symmetric difference.
    pub fn core_symdiff(&mut self) -> i32 {
        if self.stack.len() < 2 {
            self.print_error("symdiff requires two arguments");
            return 0;
        }

        let set2_rr = self.stack.pop().unwrap();
        let set1_rr = self.stack.pop().unwrap();

        match (set1_rr, set2_rr) {
            (Value::Set(s1), Value::Set(s2)) => {
                let mut new_hsh = IndexMap::new();
                for (k, v) in s1.borrow().iter() {
                    if s2.borrow().get(k).is_none() {
                        new_hsh.insert(k.clone(), v.value_clone());
                    }
                }
                for (k, v) in s2.borrow().iter() {
                    if s1.borrow().get(k).is_none() {
                        new_hsh.insert(k.clone(), v.value_clone());
                    }
                }
                let set = Value::Set(Rc::new(RefCell::new(new_hsh)));
                self.stack.push(set);
            }
            (Value::IpSet(ipset1), Value::IpSet(ipset2)) => {
                let ipset1_ipv4 = &ipset1.borrow().ipv4;
                let ipset1_ipv6 = &ipset1.borrow().ipv6;
                let ipset2_ipv4 = &ipset2.borrow().ipv4;
                let ipset2_ipv6 = &ipset2.borrow().ipv6;
                let ipv4_is = ipset1_ipv4.intersect(&ipset2_ipv4);
                let ipv6_is = ipset1_ipv6.intersect(&ipset2_ipv6);
                let new_ipv4 = ipset1_ipv4.merge(&ipset2_ipv4).exclude(&ipv4_is);
                let new_ipv6 = ipset1_ipv6.merge(&ipset2_ipv6).exclude(&ipv6_is);
                let new_ipset = IpSet::new(new_ipv4, new_ipv6);
                self.stack
                    .push(Value::IpSet(Rc::new(RefCell::new(new_ipset))));
                return 1;
            }
            (Value::Set(_), _) => {
                self.print_error("second symdiff argument must be set");
                return 0;
            }
            (_, _) => {
                self.print_error("first symdiff argument must be set");
                return 0;
            }
        }
        return 1;
    }
}
