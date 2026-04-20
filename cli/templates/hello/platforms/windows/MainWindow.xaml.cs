using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using uniffi.{{name_snake}}_ffi;

namespace {{name_pascal}}
{
    public sealed partial class MainWindow : Window
    {
        private Core core;

        public MainWindow()
        {
            this.InitializeComponent();
            core = new Core();
            GreetingText.Text = core.Greeting();
        }

        private void RefreshButton_Click(object sender, RoutedEventArgs e)
        {
            GreetingText.Text = core.Greeting();
        }
    }
}
