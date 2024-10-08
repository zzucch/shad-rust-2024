#![forbid(unsafe_code)]

use std::{collections::HashMap, fmt::Display};

////////////////////////////////////////////////////////////////////////////////

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Number(f64),
    Symbol(String),
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Number(num) => write!(f, "{}", num),
            Self::Symbol(sym) => write!(f, "'{}", sym),
        }
    }
}

////////////////////////////////////////////////////////////////////////////////

pub struct Interpreter {
    stack: Vec<Value>,
    variables: HashMap<String, Value>,
}
impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}

impl Interpreter {
    pub fn new() -> Self {
        Self {
            stack: Vec::new(),
            variables: HashMap::new(),
        }
    }

    pub fn stack(&self) -> &[Value] {
        &self.stack[..]
    }

    pub fn eval(&mut self, expr: &str) {
        let tokens: Vec<&str> = expr.split_whitespace().collect();

        for token in tokens {
            if let Ok(number) = token.parse::<f64>() {
                self.stack.push(Value::Number(number));
                continue;
            }

            match token {
                "+" => self.handle_arithmetic_operation(Self::sum),
                "-" => self.handle_arithmetic_operation(Self::subtract),
                "*" => self.handle_arithmetic_operation(Self::multiply),
                "/" => self.handle_arithmetic_operation(Self::divide),
                "set" => self.set_variable(),
                number if number.parse::<f64>().is_ok() => {
                    self.handle_number(number.parse::<f64>().unwrap())
                }
                apostrophe_variable_name
                    if apostrophe_variable_name.strip_prefix('\'').is_some() =>
                {
                    self.push_variable_name(apostrophe_variable_name.strip_prefix('\'').unwrap())
                }
                dollar_variable_name if dollar_variable_name.strip_prefix('$').is_some() => self
                    .lookup_and_push_variable_value(
                        dollar_variable_name.strip_prefix('$').unwrap(),
                    ),
                something => panic!("invalid token: {something}"),
            }
        }
    }

    fn handle_arithmetic_operation(&mut self, operation: fn(a: f64, b: f64) -> f64) {
        let value_1 = self.stack.pop();
        let value_2 = self.stack.pop();

        let operand_1 = self.get_operand_value(value_1);
        let operand_2 = self.get_operand_value(value_2);

        self.stack
            .push(Value::Number(operation(operand_1, operand_2)))
    }

    fn get_operand_value(&self, operand: Option<Value>) -> f64 {
        match operand {
            Some(Value::Number(number)) => number,
            Some(Value::Symbol(variable_name)) => match self.variables.get(&variable_name) {
                Some(Value::Number(variable_value)) => *variable_value,
                _ => panic!("variable with name '{variable_name}' does not exist"),
            },
            _ => panic!("incorrect operand"),
        }
    }

    fn set_variable(&mut self) {
        match self.stack.pop() {
            Some(Value::Symbol(variable_name)) => match self.stack.pop() {
                Some(variable_value) => {
                    self.variables.insert(variable_name, variable_value);
                }
                None => panic!(
                    "expected a value to assign to variable '{variable_name}', but stack was empty"
                ),
            },
            _ => panic!("expected a variable name on the stack, but found none"),
        }
    }

    fn push_variable_name(&mut self, name: &str) {
        self.stack.push(Value::Symbol(name.to_string()));
    }

    fn lookup_and_push_variable_value(&mut self, variable_name: &str) {
        match self.variables.get(variable_name) {
            Some(value) => self.stack.push(value.clone()),
            None => panic!("variable '{variable_name}' not found"),
        }
    }

    fn handle_number(&mut self, number: f64) {
        self.stack.push(Value::Number(number))
    }

    fn sum(a: f64, b: f64) -> f64 {
        a + b
    }

    fn subtract(a: f64, b: f64) -> f64 {
        a - b
    }

    fn multiply(a: f64, b: f64) -> f64 {
        a * b
    }

    fn divide(a: f64, b: f64) -> f64 {
        a / b
    }
}
