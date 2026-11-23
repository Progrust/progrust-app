import { expect, it } from "vitest";
import { menuItemOf } from "./router";

it("動画画面と演習画面では、メニューの「レッスン」を選ぶ", () => {
  expect(menuItemOf({ name: "home" })).toBe("home");
  expect(menuItemOf({ name: "lessons" })).toBe("lessons");
  expect(menuItemOf({ name: "video", lessonId: "ch01-01" })).toBe("lessons");
  expect(menuItemOf({ name: "exercise", lessonId: "ch01-01" })).toBe("lessons");
  expect(menuItemOf({ name: "generated" })).toBe("generated");
  expect(menuItemOf({ name: "settings" })).toBe("settings");
});
