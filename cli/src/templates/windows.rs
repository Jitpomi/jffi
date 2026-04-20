use anyhow::Result;
use colored::*;
use std::fs;
use std::path::PathBuf;

pub fn create_windows_project(platforms_dir: &PathBuf, name: &str) -> Result<()> {
    let windows_dir = platforms_dir.join("windows");
    fs::create_dir_all(&windows_dir)?;
    
    // Create WinUI 3 application files
    create_csproj(&windows_dir, name)?;
    create_app_xaml(&windows_dir, name)?;
    create_app_xaml_cs(&windows_dir, name)?;
    create_main_window_xaml(&windows_dir, name)?;
    create_main_window_xaml_cs(&windows_dir, name)?;
    create_package_appxmanifest(&windows_dir, name)?;
    create_app_manifest(&windows_dir)?;
    
    println!("  {} platforms/windows/", "✓".green());
    Ok(())
}

fn create_csproj(dir: &PathBuf, name: &str) -> Result<()> {
    let class_name = name.split('-').map(|s| {
        let mut c = s.chars();
        match c.next() {
            None => String::new(),
            Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        }
    }).collect::<Vec<_>>().join("");
    
    let content = format!(r#"<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <OutputType>WinExe</OutputType>
    <TargetFramework>net8.0-windows10.0.19041.0</TargetFramework>
    <TargetPlatformMinVersion>10.0.17763.0</TargetPlatformMinVersion>
    <RootNamespace>{}</RootNamespace>
    <ApplicationManifest>app.manifest</ApplicationManifest>
    <Platforms>x64;ARM64</Platforms>
    <RuntimeIdentifiers>win-x64;win-arm64</RuntimeIdentifiers>
    <PublishProfile>win-$(Platform).pubxml</PublishProfile>
    <UseWinUI>true</UseWinUI>
    <EnableMsixTooling>true</EnableMsixTooling>
    <AllowUnsafeBlocks>true</AllowUnsafeBlocks>
  </PropertyGroup>

  <ItemGroup>
    <Content Include="Assets\**" />
    <Content Include="*_ffi.dll">
      <CopyToOutputDirectory>PreserveNewest</CopyToOutputDirectory>
    </Content>
  </ItemGroup>

  <ItemGroup>
    <PackageReference Include="Microsoft.WindowsAppSDK" Version="1.5.240802000" />
    <PackageReference Include="Microsoft.Windows.SDK.BuildTools" Version="10.0.26100.1742" />
  </ItemGroup>

  <ItemGroup>
    <Manifest Include="$(ApplicationManifest)" />
  </ItemGroup>
</Project>
"#, class_name);
    
    fs::write(dir.join(format!("{}.csproj", name)), content)?;
    Ok(())
}

fn create_app_xaml(dir: &PathBuf, name: &str) -> Result<()> {
    let class_name = name.split('-').map(|s| {
        let mut c = s.chars();
        match c.next() {
            None => String::new(),
            Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        }
    }).collect::<Vec<_>>().join("");
    
    let content = format!(r#"<Application
    x:Class="{}.App"
    xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation"
    xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml">
    <Application.Resources>
        <ResourceDictionary>
            <ResourceDictionary.MergedDictionaries>
                <XamlControlsResources xmlns="using:Microsoft.UI.Xaml.Controls" />
            </ResourceDictionary.MergedDictionaries>
        </ResourceDictionary>
    </Application.Resources>
</Application>
"#, class_name);
    
    fs::write(dir.join("App.xaml"), content)?;
    Ok(())
}

fn create_app_xaml_cs(dir: &PathBuf, name: &str) -> Result<()> {
    let class_name = name.split('-').map(|s| {
        let mut c = s.chars();
        match c.next() {
            None => String::new(),
            Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        }
    }).collect::<Vec<_>>().join("");
    
    let content = format!(r#"using Microsoft.UI.Xaml;

namespace {}
{{
    public partial class App : Application
    {{
        public App()
        {{
            this.InitializeComponent();
        }}

        protected override void OnLaunched(Microsoft.UI.Xaml.LaunchActivatedEventArgs args)
        {{
            m_window = new MainWindow();
            m_window.Activate();
        }}

        private Window m_window;
    }}
}}
"#, class_name);
    
    fs::write(dir.join("App.xaml.cs"), content)?;
    Ok(())
}

fn create_main_window_xaml(dir: &PathBuf, name: &str) -> Result<()> {
    let class_name = name.split('-').map(|s| {
        let mut c = s.chars();
        match c.next() {
            None => String::new(),
            Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        }
    }).collect::<Vec<_>>().join("");
    
    let content = format!(r#"<Window
    x:Class="{}.MainWindow"
    xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation"
    xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"
    xmlns:d="http://schemas.microsoft.com/expression/blend/2008"
    xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006"
    mc:Ignorable="d">

    <Grid>
        <Grid.RowDefinitions>
            <RowDefinition Height="Auto"/>
            <RowDefinition Height="Auto"/>
            <RowDefinition Height="*"/>
        </Grid.RowDefinitions>

        <!-- Header -->
        <Grid Grid.Row="0" Background="{{ThemeResource CardBackgroundFillColorDefaultBrush}}" Padding="24">
            <Grid.ColumnDefinitions>
                <ColumnDefinition Width="*"/>
                <ColumnDefinition Width="Auto"/>
            </Grid.ColumnDefinitions>
            <TextBlock Text="Today" FontSize="28" FontWeight="SemiBold" Grid.Column="0"/>
            <Button x:Name="AddButton" Content="+ Add Task" Grid.Column="1" Click="AddButton_Click"/>
        </Grid>

        <!-- Stats -->
        <Grid Grid.Row="1" Padding="24" Background="{{ThemeResource LayerFillColorDefaultBrush}}">
            <Grid.ColumnDefinitions>
                <ColumnDefinition Width="*"/>
                <ColumnDefinition Width="*"/>
                <ColumnDefinition Width="*"/>
            </Grid.ColumnDefinitions>

            <StackPanel Grid.Column="0" Padding="12" Background="{{ThemeResource CardBackgroundFillColorDefaultBrush}}" CornerRadius="8" Margin="0,0,8,0">
                <TextBlock x:Name="TotalCount" FontSize="32" FontWeight="Bold" HorizontalAlignment="Center"/>
                <TextBlock Text="Total" FontSize="14" Foreground="{{ThemeResource TextFillColorSecondaryBrush}}" HorizontalAlignment="Center"/>
            </StackPanel>

            <StackPanel Grid.Column="1" Padding="12" Background="{{ThemeResource CardBackgroundFillColorDefaultBrush}}" CornerRadius="8" Margin="0,0,8,0">
                <TextBlock x:Name="ActiveCount" FontSize="32" FontWeight="Bold" HorizontalAlignment="Center"/>
                <TextBlock Text="Active" FontSize="14" Foreground="{{ThemeResource TextFillColorSecondaryBrush}}" HorizontalAlignment="Center"/>
            </StackPanel>

            <StackPanel Grid.Column="2" Padding="12" Background="{{ThemeResource CardBackgroundFillColorDefaultBrush}}" CornerRadius="8">
                <TextBlock x:Name="DoneCount" FontSize="32" FontWeight="Bold" HorizontalAlignment="Center"/>
                <TextBlock Text="Done" FontSize="14" Foreground="{{ThemeResource TextFillColorSecondaryBrush}}" HorizontalAlignment="Center"/>
            </StackPanel>
        </Grid>

        <!-- Tasks List -->
        <ScrollViewer Grid.Row="2" Padding="24">
            <ItemsControl x:Name="TasksList">
                <ItemsControl.ItemTemplate>
                    <DataTemplate>
                        <Grid Padding="16" Background="{{ThemeResource LayerFillColorDefaultBrush}}" CornerRadius="8" Margin="0,0,0,8">
                            <Grid.ColumnDefinitions>
                                <ColumnDefinition Width="Auto"/>
                                <ColumnDefinition Width="*"/>
                            </Grid.ColumnDefinitions>
                            <CheckBox Grid.Column="0" IsChecked="{{Binding Completed}}" Margin="0,0,12,0" Click="TaskCheckBox_Click" Tag="{{Binding Id}}"/>
                            <TextBlock Grid.Column="1" Text="{{Binding Title}}" VerticalAlignment="Center"/>
                        </Grid>
                    </DataTemplate>
                </ItemsControl.ItemTemplate>
            </ItemsControl>
        </ScrollViewer>
    </Grid>
</Window>
"#, class_name);
    
    fs::write(dir.join("MainWindow.xaml"), content)?;
    Ok(())
}

fn create_main_window_xaml_cs(dir: &PathBuf, name: &str) -> Result<()> {
    let class_name = name.split('-').map(|s| {
        let mut c = s.chars();
        match c.next() {
            None => String::new(),
            Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        }
    }).collect::<Vec<_>>().join("");
    
    let content = format!(r#"using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using System;
using System.Collections.ObjectModel;
using System.Linq;
using uniffi.{}_ffi;

namespace {}
{{
    public class TaskItem
    {{
        public string Id {{ get; set; }}
        public string Title {{ get; set; }}
        public bool Completed {{ get; set; }}
    }}

    public sealed partial class MainWindow : Window
    {{
        private ObservableCollection<TaskItem> tasks = new ObservableCollection<TaskItem>();
        private Core core;

        public MainWindow()
        {{
            this.InitializeComponent();
            core = new Core();
            TasksList.ItemsSource = tasks;
            LoadTasks();
            UpdateStats();
        }}

        private void LoadTasks()
        {{
            tasks.Clear();
            var items = core.GetItems();
            foreach (var item in items)
            {{
                tasks.Add(new TaskItem
                {{
                    Id = item.id,
                    Title = item.title,
                    Completed = item.completed
                }});
            }}
        }}

        private void AddButton_Click(object sender, RoutedEventArgs e)
        {{
            ShowAddTaskDialog();
        }}

        private async void ShowAddTaskDialog()
        {{
            var dialog = new ContentDialog
            {{
                Title = "New Task",
                PrimaryButtonText = "Add",
                CloseButtonText = "Cancel",
                DefaultButton = ContentDialogButton.Primary,
                XamlRoot = this.Content.XamlRoot
            }};

            var textBox = new TextBox
            {{
                PlaceholderText = "Enter task name..."
            }};
            dialog.Content = textBox;

            var result = await dialog.ShowAsync();
            if (result == ContentDialogResult.Primary && !string.IsNullOrWhiteSpace(textBox.Text))
            {{
                var id = Guid.NewGuid().ToString();
                var items = core.AddItem(id, textBox.Text);
                
                tasks.Clear();
                foreach (var item in items)
                {{
                    tasks.Add(new TaskItem
                    {{
                        Id = item.id,
                        Title = item.title,
                        Completed = item.completed
                    }});
                }}
                UpdateStats();
            }}
        }}

        private void TaskCheckBox_Click(object sender, RoutedEventArgs e)
        {{
            if (sender is CheckBox checkBox && checkBox.Tag is string id)
            {{
                var items = core.ToggleItem(id);
                tasks.Clear();
                foreach (var item in items)
                {{
                    tasks.Add(new TaskItem
                    {{
                        Id = item.id,
                        Title = item.title,
                        Completed = item.completed
                    }});
                }}
                UpdateStats();
            }}
        }}

        private void UpdateStats()
        {{
            TotalCount.Text = tasks.Count.ToString();
            ActiveCount.Text = tasks.Count(t => !t.Completed).ToString();
            DoneCount.Text = tasks.Count(t => t.Completed).ToString();
        }}
    }}
}}
"#, name.replace("-", "_"), class_name);
    
    fs::write(dir.join("MainWindow.xaml.cs"), content)?;
    Ok(())
}

fn create_package_appxmanifest(dir: &PathBuf, name: &str) -> Result<()> {
    let class_name = name.split('-').map(|s| {
        let mut c = s.chars();
        match c.next() {
            None => String::new(),
            Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        }
    }).collect::<Vec<_>>().join("");
    
    let content = format!(r#"<?xml version="1.0" encoding="utf-8"?>
<Package
  xmlns="http://schemas.microsoft.com/appx/manifest/foundation/windows10"
  xmlns:mp="http://schemas.microsoft.com/appx/2014/phone/manifest"
  xmlns:uap="http://schemas.microsoft.com/appx/manifest/uap/windows10"
  xmlns:rescap="http://schemas.microsoft.com/appx/manifest/foundation/windows10/restrictedcapabilities"
  IgnorableNamespaces="uap rescap">

  <Identity
    Name="com.{}.{}"
    Publisher="CN={}"
    Version="1.0.0.0" />

  <mp:PhoneIdentity PhoneProductId="00000000-0000-0000-0000-000000000000" PhonePublisherId="00000000-0000-0000-0000-000000000000"/>

  <Properties>
    <DisplayName>{}</DisplayName>
    <PublisherDisplayName>{}</PublisherDisplayName>
    <Logo>Assets\StoreLogo.png</Logo>
  </Properties>

  <Dependencies>
    <TargetDeviceFamily Name="Windows.Universal" MinVersion="10.0.17763.0" MaxVersionTested="10.0.19041.0" />
    <TargetDeviceFamily Name="Windows.Desktop" MinVersion="10.0.17763.0" MaxVersionTested="10.0.19041.0" />
  </Dependencies>

  <Resources>
    <Resource Language="x-generate"/>
  </Resources>

  <Applications>
    <Application Id="App"
      Executable="$targetnametoken$.exe"
      EntryPoint="$targetentrypoint$">
      <uap:VisualElements
        DisplayName="{}"
        Description="{}"
        BackgroundColor="transparent"
        Square150x150Logo="Assets\Square150x150Logo.png"
        Square44x44Logo="Assets\Square44x44Logo.png">
        <uap:DefaultTile Wide310x150Logo="Assets\Wide310x150Logo.png" />
        <uap:SplashScreen Image="Assets\SplashScreen.png" />
      </uap:VisualElements>
    </Application>
  </Applications>

  <Capabilities>
    <rescap:Capability Name="runFullTrust" />
  </Capabilities>
</Package>
"#, name, name, class_name, class_name, class_name, class_name, class_name);
    
    fs::write(dir.join("Package.appxmanifest"), content)?;
    Ok(())
}

fn create_app_manifest(dir: &PathBuf) -> Result<()> {
    let content = r#"<?xml version="1.0" encoding="utf-8"?>
<assembly manifestVersion="1.0" xmlns="urn:schemas-microsoft-com:asm.v1">
  <assemblyIdentity version="1.0.0.0" name="MyApplication.app"/>
  <compatibility xmlns="urn:schemas-microsoft-com:compatibility.v1">
    <application>
      <!-- Windows 10 and Windows 11 -->
      <supportedOS Id="{8e0f7a12-bfb3-4fe8-b9a5-48fd50a15a9a}" />
    </application>
  </compatibility>
  
  <application xmlns="urn:schemas-microsoft-com:asm.v3">
    <windowsSettings>
      <dpiAware xmlns="http://schemas.microsoft.com/SMI/2005/WindowsSettings">true</dpiAware>
      <dpiAwareness xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">PerMonitorV2</dpiAwareness>
    </windowsSettings>
  </application>
</assembly>
"#;
    
    fs::write(dir.join("app.manifest"), content)?;
    Ok(())
}
