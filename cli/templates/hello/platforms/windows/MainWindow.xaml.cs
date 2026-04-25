using Microsoft.UI.Xaml;
using uniffi.{{name_snake}}_core;

namespace {{name_pascal}}
{
    public sealed partial class MainWindow : Window
    {
        private readonly Core _core = new();

        public MainWindow()
        {
            this.InitializeComponent();
            Title = "Hello from JFFI";
            GreetingText.Text = _core.Greeting();
        }

        private void RefreshButton_Click(object sender, RoutedEventArgs e)
        {
            GreetingText.Text = _core.Greeting();
        }
    }
}
