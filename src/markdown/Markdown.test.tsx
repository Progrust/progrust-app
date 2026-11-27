import { render } from "@testing-library/react";
import { expect, it } from "vitest";

import { Markdown } from "./Markdown";

const SOURCE = "```rust\nfn main() {}\n```";

it("色分けを求めなければ、Rust のコードブロックに色のクラスを付けない", () => {
  const { container } = render(<Markdown source={SOURCE} />);
  expect(container.querySelector(".tk-k")).toBeNull();
  expect(container.querySelector("pre.pg-code")).not.toBeNull();
});

it("色分けを求めると、Rust のコードブロックだけに色のクラスを付ける", () => {
  const { container } = render(
    <Markdown
      source={`${SOURCE}\n\n\`\`\`text\nfn\n\`\`\`\n\n\`fn\``}
      highlight
    />,
  );
  expect(container.querySelectorAll(".tk-k")).toHaveLength(1);
});
