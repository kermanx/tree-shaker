class A {
  constructor(a) {
    this.a = a;
  }
  method(x) {
    return x + 1
  }
  _g
  get g() {
    return this._g;
  }
  set g(value) {
    this._g = value + 2;
  }

  static create() {
    return new A(100);
  }
  get has_effect() {
    return unknown()
  }

  _unused() {
    console.log('unused');
  }
  static _unusedStatic() {
    console.log('unused static');
  }
}

const a = new A(1);
console.log(a.a, a.g = 10, a.g, a.method(20))

const b = A.create();
b.has_effect
