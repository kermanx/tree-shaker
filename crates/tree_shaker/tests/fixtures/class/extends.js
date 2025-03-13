let effects = ""
class Base {
  constructor(x) {
    effects += "Base" + x
  }
  base1 = 1
  base2 = 2
  run() {
    console.log('Base')
  }
}
class Derived extends Base {
  constructor() {
    super(1)
    effects += "Derived"
  }
  base2 = 3
  derived1 = 4
  run() {
    console.log('Derived')
  }
}

const a = new Derived()
console.log(effects)
console.log(a.base1, a.base2, a.derived1)

a.run()
