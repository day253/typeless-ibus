import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import App from "./App";
import Capsule from "./Capsule";
import "./styles.css";

const Component = window.location.hash === "#capsule" ? Capsule : App;

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <Component />
  </StrictMode>,
);
