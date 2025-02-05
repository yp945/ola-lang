# Smart contracts

Ola contracts allow users to write complex business logic that will be deployed to Ola's l2 network, 
and cross-contract calls can be written between different contracts just like solidity.

## Examples

The following example shows a recursive and non-recursive ola smart contract implementation of Fibonacci numbers.

````javascript

contract Fibonacci {

    fn main() {
       fib_non_recursive(10);
    }

    fn fib_recursive(u32 n) -> (u32) {
        if (n <= 2) {
            return 1;
        }
        return fib_recursive(n -1) + fib_recursive(n -2);
    }

    fn fib_non_recursive(u32 n) -> (u32) {
        u32 first = 0;
        u32 second = 1;
        u32 third = 1;
        for (u32 i = 2; i <= n; i++) {
             third = first + second;
             first = second;
             second = third;
        }
        return third;
    }

}

````

The following shows a simple Person contract that contains a person structure, 
assigns a value to the person structure and reads the status of the person.

````javascript

contract Person {

    enum Sex {
        Man,
        Women
    }

    struct Person {
        Sex s;
        u32 age;
        u256 id;
    }

    Person p;

    fn newPerson(Sex s, u32 age, u256 id) {
        p = Person(s, age, id);
    }

    fn getPersonId() -> (u256) {
        return p.id;
    }

    fn getAge() -> (u32) {
        return p.age;
    }
}

````


## Multiple files

For better project organisation and clearer logic, 
it is common to split the contents of a file into multiple files.
ola language supports the import of another contract within a contract through the `import` keyword.

An example of a multi-file contract is shown below.


**Contract RectangularCalculator**
````javascript

contract RectangularCalculator {
  
    fn rectangle(u32 w, u32 h) -> (u32 s, u32 p) {
        s = w * h;
        p = 2 * (w + h);
        // Returns a variable with the same name, return can be ignore
        //return (s, p)
    }
}

````

**Contract ShapeCalculator**
````javascript

contract SquareCalculator {

    fn square(u32 w) -> (u32 s, u32 p) {
        s = w * w;
        p = 4 * w;
        return (s, p);
    }
}

````

**Contract Calculator**
```javascript

import "./RectangularCalculator";
import "./SquareCalculator";

contract Calculator {
  
    fn sum(u32 w, u32 h) -> (u32 s, u32 p) {
        (u32 rectangle_s, u32  rectangle_p) = rectangle(w, h);
        (u32 square_s, u32 square_p) = square(w);
        return (rectangle_s + square_s, rectangle_p + square_p);
    }
}

```


## More Features

* String and Mapping Support - More data types can be combined.
* Library functions  - Support for native fields and cryptography-related library functions.
* Cross-contract calls - Support for more complex business logic.
* ... - Coming soon.