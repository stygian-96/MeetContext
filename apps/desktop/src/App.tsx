import { Captures } from "./screens/Captures";
import { Overlay } from "./screens/Overlay";

export default function App() {
  return new URLSearchParams(location.search).get("view") === "overlay" ? <Overlay /> : <Captures />;
}
