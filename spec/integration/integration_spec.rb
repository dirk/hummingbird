RSpec.describe 'Integration' do
  directories = Dir[File.join(File.dirname(__FILE__), '*')]
    .select { |entry| File.directory?(entry) }

  def expect_runs(source_file, expected_output)
    executable = File.expand_path('../../../target/debug/hummingbird', __FILE__)
    
    output = `#{executable} #{source_file}`
    if $?.exitstatus != 0
      $stderr.puts "Command failed: #{command}"
      $stderr.puts output
      expect($?.exitstatus).to eq(0)
    end
    expect(output).to eq(expected_output)
  end
  
  directories.each do |directory|
    describe File.basename(directory) do
      it 'passes' do
        file = File.join(directory, 'test.hb')
        expected_output = File.read(File.join(directory, 'out'))
        expect_runs(file, expected_output)
      end
    end
  end
end
