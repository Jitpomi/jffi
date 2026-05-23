using Microsoft.UI.Xaml;
using System;
using uniffi.{{name_snake}}_core;

// To learn more about WinUI, the WinUI project structure,
// and more about our project templates, see: http://aka.ms/winui-project-info.

namespace {{name_pascal}}
{
    /// <summary>
    /// Main window that hosts the application UI with ViewModel-based data binding.
    /// </summary>
    public sealed partial class MainWindow : Window
    {
        public AppViewModel ViewModel { get; }

        public MainWindow()
        {
            this.InitializeComponent();

            // Set window icon
            try
            {
                var iconPath = FindIconPath();
                if (iconPath != null)
                {
                    var hWnd = WinRT.Interop.WindowNative.GetWindowHandle(this);
                    var windowId = Microsoft.UI.Win32Interop.GetWindowIdFromWindow(hWnd);
                    var appWindow = Microsoft.UI.Windowing.AppWindow.GetFromWindowId(windowId);
                    appWindow.SetIcon(iconPath);
                }
                else
                {
                    System.Diagnostics.Debug.WriteLine("Failed to find app.ico in any candidate path.");
                }
            }
            catch (Exception ex)
            {
                System.Diagnostics.Debug.WriteLine($"Failed to set window icon: {ex.Message}");
            }

            ViewModel = new AppViewModel();
        }

        private string? FindIconPath()
        {
            var candidates = new[]
            {
                System.IO.Path.Combine(System.AppContext.BaseDirectory, "Assets", "app.ico"),
                System.IO.Path.Combine(System.AppContext.BaseDirectory, "AppX", "Assets", "app.ico"),
                System.IO.Path.Combine(System.IO.Directory.GetCurrentDirectory(), "Assets", "app.ico"),
                System.IO.Path.Combine(System.IO.Directory.GetCurrentDirectory(), "platforms", "windows", "Assets", "app.ico")
            };

            foreach (var path in candidates)
            {
                if (System.IO.File.Exists(path))
                {
                    return path;
                }
            }
            return null;
        }
    }
}
