using System;
using System.ComponentModel;
using System.Windows.Input;
using uniffi.{{name_snake}}_core;

namespace {{name_pascal}}
{
    /// <summary>
    /// ViewModel that bridges WinUI XAML with the Rust Core library.
    /// Implements INotifyPropertyChanged for data binding.
    /// </summary>
    public class AppViewModel : INotifyPropertyChanged
    {
        private readonly Core _core;
        private string _greeting = "";

        public event PropertyChangedEventHandler? PropertyChanged;

        public string Greeting
        {
            get => _greeting;
            private set
            {
                if (_greeting != value)
                {
                    _greeting = value;
                    OnPropertyChanged(nameof(Greeting));
                }
            }
        }

        public ICommand RefreshGreetingCommand { get; }

        public AppViewModel()
        {
            _core = new Core();
            Greeting = _core.Greeting();
            RefreshGreetingCommand = new RelayCommand(OnRefreshGreeting);
        }

        private void OnRefreshGreeting()
        {
            Greeting = _core.Greeting();
        }

        private void OnPropertyChanged(string propertyName)
        {
            PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(propertyName));
        }
    }

    /// <summary>
    /// Simple ICommand implementation for XAML binding.
    /// </summary>
    public class RelayCommand : ICommand
    {
        private readonly Action _execute;

        public RelayCommand(Action execute)
        {
            _execute = execute;
        }

        public event EventHandler? CanExecuteChanged;

        public bool CanExecute(object? parameter) => true;

        public void Execute(object? parameter) => _execute();
    }
}
