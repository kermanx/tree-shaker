function A(x) {
  this.a = x + 1;
}

A.prototype.method = function (x) {
  return x + 1;
}
A.prototype._g = 0;

Object.defineProperty(A.prototype, "g", {
  get: function () {
    return this._g;
  },
  set: function (value) {
    this._g = value + 2;
  }
});

A.create = function () {
  return new A(100);
}

A.prototype.has_effect = function () {
  return unknown();
}

A.prototype._unused = function () {
  console.log('unused');
}
A._unusedStatic = function () {
  console.log('unused static');
}

const a = new A(1);
console.log(a.a, a.g = 10, a.g, a.method(20))
const b = A.create();
b.has_effect()
