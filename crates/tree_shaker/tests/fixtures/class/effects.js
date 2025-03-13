class _C {
  static a = effect(1);
  [effect(2)] = effect(3);
  static [effect(4)] = effect(5);
  static {
    effect(6);
  }
  [effect(7)]() {
    effect(8);
  }
  static {
    effect(9);
  }
}
