using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using uniffi.{{name_snake}}_core;

namespace {{name_pascal}}
{
    public sealed class MainWindow : Window
    {
        private readonly Core _core = new();
        private readonly TextBlock _greetingText = new()
        {
            FontSize = 24,
            FontWeight = Microsoft.UI.Text.FontWeights.SemiBold,
            TextAlignment = TextAlignment.Center,
        };

        public MainWindow()
        {
            Title = "Hello from JFFI";

            var refreshButton = new Button
            {
                Content = "Refresh",
            };
            refreshButton.Click += RefreshButton_Click;

            var stack = new StackPanel
            {
                VerticalAlignment = VerticalAlignment.Center,
                HorizontalAlignment = HorizontalAlignment.Center,
                Spacing = 16,
            };
            stack.Children.Add(_greetingText);
            stack.Children.Add(refreshButton);

            Content = stack;
            _greetingText.Text = _core.Greeting();
        }

        private void RefreshButton_Click(object sender, RoutedEventArgs e)
        {
            _greetingText.Text = _core.Greeting();
        }
    }
}
