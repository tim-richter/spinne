import { expect, it } from "vitest";
import { getErrorMessage } from "./error.js";

it("should return error as string if not instanceof Error", () => {
  const error = getErrorMessage("My custom error");

  expect(error).toEqual("My custom error");
});

it("should return other data types as string if not instanceof Error", () => {
  const error = getErrorMessage(["My custom error"]);

  expect(error).toEqual("My custom error");
});

it("should return message if typeof Error", () => {
  const error = getErrorMessage(new Error("My instanceOf Error"));

  expect(error).toEqual("My instanceOf Error");
});
