using Microsoft.UI.Xaml;
using WinRT;

namespace {{name_pascal}}
{
    public sealed class App : Application
    {
        [System.STAThread]
        public static void Main(string[] args)
        {
            ComWrappersSupport.InitializeComWrappers();
            Application.Start(_ => new App());
        }

        public App()
        {
        }

        protected override void OnLaunched(Microsoft.UI.Xaml.LaunchActivatedEventArgs args)
        {
            _window = new MainWindow();
            _window.Activate();
        }

        private Window _window = null!;
    }
}
