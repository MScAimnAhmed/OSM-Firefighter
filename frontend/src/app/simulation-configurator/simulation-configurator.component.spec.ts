import { ComponentFixture, TestBed } from '@angular/core/testing';

import { SimulationConfiguratorComponent } from './simulation-configurator.component';

describe('SimulationConfiguratorComponent', () => {
  let component: SimulationConfiguratorComponent;
  let fixture: ComponentFixture<SimulationConfiguratorComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      declarations: [ SimulationConfiguratorComponent ]
    })
    .compileComponents();
  });

  beforeEach(() => {
    fixture = TestBed.createComponent(SimulationConfiguratorComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });
});
