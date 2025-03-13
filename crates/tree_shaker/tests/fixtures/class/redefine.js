function A(x) {
  this.x = x
}
A.prototype = {
  constructor: A,
  method: function (y) {
    return this.x + y;
  },
}

const a = new A(1);
console.log(a.method(2));
