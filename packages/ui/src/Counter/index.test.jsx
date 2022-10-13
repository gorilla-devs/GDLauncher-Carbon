import { describe, it, expect } from "vitest";
import { render, screen } from "solid-testing-library";
import { Counter } from ".";

describe("Timer component", () => {
  it("should assert some dummy assertion", () => {
    render(<Counter />);
    const timerElm = screen.getByText("Clickk");
    expect(timerElm).toBeInTheDocument();
  });
});
