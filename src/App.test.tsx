import { render, screen } from "@testing-library/react";
import { expect, it } from "vitest";
import { App } from "./App";

it("仮のページを表示する", () => {
  render(<App />);
  expect(
    screen.getByRole("heading", { level: 1, name: "Progrust" }),
  ).toBeInTheDocument();
});
