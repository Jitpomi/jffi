using Microsoft.UI.Xaml;
using uniffi.{{name_snake}}_core;

namespace {{name_pascal}}
{
    /// <summary>
    /// An empty window that can be used on its own or navigated to within a Frame.
    /// </summary>
    public sealed partial class MainWindow : Window
    {
        private readonly Core _core = new();

        public MainWindow()
        {
            InitializeComponent();
            GreetingText.Text = _core.Greeting();
        }

        private void RefreshButton_Click(object sender, RoutedEventArgs e)
        {
            GreetingText.Text = _core.Greeting();
        }
    }
}