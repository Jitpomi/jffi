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
    mc:Ignorable="d"
    Title="Hello from JFFI"
    Width="600"
    Height="400">

    <Grid Background="{{ThemeResource LayerFillColorDefaultBrush}}">
        <StackPanel VerticalAlignment="Center" HorizontalAlignment="Center" Spacing="16">
            <TextBlock x:Name="GreetingText" FontSize="24" FontWeight="SemiBold" TextAlignment="Center"/>
            <Button x:Name="RefreshButton" Content="Refresh" Click="RefreshButton_Click" Style="{{ThemeResource AccentButtonStyle}}"/>
        </StackPanel>
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
using uniffi.{}_ffi;

namespace {}
{{
    public sealed partial class MainWindow : Window
    {{
        private Core core;

        public MainWindow()
        {{
            this.InitializeComponent();
            core = new Core();
            GreetingText.Text = core.Greeting();
        }}

        private void RefreshButton_Click(object sender, RoutedEventArgs e)
        {{
            GreetingText.Text = core.Greeting();
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
