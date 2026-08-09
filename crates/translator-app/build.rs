fn main() {
    // Framework-dependent: stage Bootstrap.dll + resources.pri next to the binary.
    // Target machine must have a matching Windows App Runtime installed.
    windows_reactor_setup::as_framework_dependent();
}
