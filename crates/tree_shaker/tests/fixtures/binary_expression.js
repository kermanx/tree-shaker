export const strict_equality = (a, b) => ({
  s1: 1 === 1,
  s2: 1 === "1",
  s3: unknown === 1,
  s4: a === a,
  s5: typeof (a === unknown),
})

export const add = {
  s1: 1 + 1,
  s2: 1 + "a",
  s3: 1 + true,
  s4: 1 + null,
  s5: "a" + 1,
  s6: "a" + "a",
  s7: "a" + true,
  s8: {} + {},
}
