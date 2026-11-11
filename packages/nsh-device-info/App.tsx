import { mount, Text, View } from "nsh";

// NSH 0.4.0's entry resolution requires an App.tsx beside nsh.json for `nsh check` and `nsh test`.
// This package is a library with no demo, so the entry stays as empty as the toolchain allows.
function App() {
  return (
    <View class="flex-1 items-center justify-center">
      <Text class="text-foreground">nsh-device-info</Text>
    </View>
  );
}

mount(App);
