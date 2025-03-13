export const A = class X {
  [1+1] = 1+1
  get [3+3]() { return 3+3 }
  set x(v) { this._x = v }
};
export const B = class extends unknown(1+1) {
  constructor(a, b) {
    super(a);
    b();
  }
  fn(a) {
    void a;
    a = console.log;
    a();
  }
};
export default class {
  a = 1;
}
