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
            ViewModel = new AppViewModel();
        }
    }
}
